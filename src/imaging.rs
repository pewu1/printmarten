use anyhow::{Context, Result};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbImage};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct Source {
    pub path: PathBuf,
    pub pixels: Arc<RgbImage>,
}

pub fn load(path: &Path) -> Result<Source> {
    let heif = path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        ["heic", "heif", "hif"]
            .iter()
            .any(|e| s.eq_ignore_ascii_case(e))
    });
    let (mut pixels, profile) = if heif {
        heic::decode(path)?
    } else {
        let mut reader = ImageReader::open(path)?.with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(30000);
        limits.max_image_height = Some(30000);
        limits.max_alloc = Some(512 * 1024 * 1024);
        reader.limits(limits);
        let mut decoder = reader.into_decoder()?;
        let orientation = decoder.orientation()?;
        let profile = decoder.icc_profile()?;
        let mut image = DynamicImage::from_decoder(decoder)?;
        image.apply_orientation(orientation);
        let mut rgba = image.to_rgba8();
        for p in rgba.pixels_mut() {
            let alpha = p[3] as u32;
            for c in &mut p.0[..3] {
                *c = ((*c as u32 * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
            }
            p[3] = 255;
        }
        (DynamicImage::ImageRgba8(rgba).into_rgb8(), profile)
    };
    if let Some(profile) = profile {
        let input = lcms2::Profile::new_icc(&profile).context("Invalid embedded color profile")?;
        if input.color_space() == lcms2::ColorSpaceSignature::RgbData {
            let transform = lcms2::Transform::<u8, u8>::new(
                &input,
                lcms2::PixelFormat::RGB_8,
                &lcms2::Profile::new_srgb(),
                lcms2::PixelFormat::RGB_8,
                lcms2::Intent::Perceptual,
            )?;
            transform.transform_in_place(pixels.as_mut());
        }
    }
    Ok(Source {
        path: path.to_owned(),
        pixels: Arc::new(pixels),
    })
}

pub fn effective_dpi(pixels: (u32, u32), placement: &crate::layout::Placement) -> f64 {
    let (w, h) = if placement.rotated {
        (pixels.1, pixels.0)
    } else {
        pixels
    };
    let x = w as f64 / (placement.rect.w as f64 / 25400.0);
    let y = h as f64 / (placement.rect.h as f64 / 25400.0);
    match placement.fit {
        crate::model::Fit::Fill => x.min(y),
        crate::model::Fit::Contain => x.max(y),
    }
}

/// Load the bundled HEIC decoder at runtime so --version and non-HEIC jobs work
/// even when a development machine has not built the optional native libraries.
mod heic {
    use super::*;
    use libloading::Library;
    use std::ffi::{CStr, c_char, c_int, c_void};
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Error {
        code: c_int,
        subcode: c_int,
        message: *const c_char,
    }
    impl Error {
        fn check(self) -> Result<()> {
            if self.code == 0 {
                return Ok(());
            }
            let message = if self.message.is_null() {
                "HEIC decoding failed".to_owned()
            } else {
                unsafe { CStr::from_ptr(self.message) }
                    .to_string_lossy()
                    .into_owned()
            };
            anyhow::bail!("{message}")
        }
    }
    type Ptr = *mut c_void;
    type Init = unsafe extern "C" fn(*const c_void) -> Error;
    type Deinit = unsafe extern "C" fn();
    type Alloc = unsafe extern "C" fn() -> Ptr;
    type Release = unsafe extern "C" fn(Ptr);
    type Read = unsafe extern "C" fn(Ptr, *const c_void, usize, *const c_void) -> Error;
    type Primary = unsafe extern "C" fn(Ptr, *mut Ptr) -> Error;
    type Dimension = unsafe extern "C" fn(Ptr) -> c_int;
    type Decode = unsafe extern "C" fn(Ptr, *mut Ptr, c_int, c_int, *const c_void) -> Error;
    type Plane = unsafe extern "C" fn(Ptr, c_int, *mut c_int) -> *const u8;
    type ProfileSize = unsafe extern "C" fn(Ptr) -> usize;
    type Profile = unsafe extern "C" fn(Ptr, *mut c_void) -> Error;

    struct Guard {
        ptr: Ptr,
        release: Release,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe { (self.release)(self.ptr) };
        }
    }
    struct Initialized(Deinit);
    impl Drop for Initialized {
        fn drop(&mut self) {
            unsafe { (self.0)() };
        }
    }

    pub fn decode(path: &Path) -> Result<(RgbImage, Option<Vec<u8>>)> {
        let exe = std::env::current_exe()?;
        let dir = exe.parent().context("Executable has no directory")?;
        let names: &[&str] = if cfg!(target_os = "macos") {
            &["libheif.dylib", "libheif.1.dylib"]
        } else if cfg!(windows) {
            &["heif.dll", "libheif.dll"]
        } else {
            &["libheif.so.1", "libheif.so"]
        };
        let mut candidates = Vec::new();
        if let Some(native) = std::env::var_os("PRINTMARTEN_NATIVE_LIB_DIR") {
            for name in names {
                candidates.push(PathBuf::from(&native).join(name));
            }
        }
        for base in [
            dir.to_owned(),
            dir.join("lib"),
            dir.join("../Frameworks"),
            dir.join("../native/lib"),
            dir.join("../native/bin"),
            dir.join("../../native/lib"),
            dir.join("../../native/bin"),
        ] {
            for name in names {
                candidates.push(base.join(name));
            }
        }
        // Development fallback to the system installation.
        for name in names {
            candidates.push(PathBuf::from(name));
        }
        let lib = candidates.iter().find_map(|p| open_library(p).ok())
            .context("HEIC decoder is unavailable. Use an official package or build native dependencies (see README).")?;
        // SAFETY: these signatures match libheif's stable C ABI. All handles and
        // the initialization guard are dropped before the library is unloaded.
        unsafe {
            let init: Init = *lib.get(b"heif_init\0")?;
            init(std::ptr::null()).check()?;
            let _init = Initialized(*lib.get(b"heif_deinit\0")?);
            let alloc: Alloc = *lib.get(b"heif_context_alloc\0")?;
            let context = Guard {
                ptr: alloc(),
                release: *lib.get(b"heif_context_free\0")?,
            };
            anyhow::ensure!(!context.ptr.is_null(), "HEIC allocation failed");
            let bytes = std::fs::read(path)?;
            let read: Read = *lib.get(b"heif_context_read_from_memory\0")?;
            read(
                context.ptr,
                bytes.as_ptr().cast(),
                bytes.len(),
                std::ptr::null(),
            )
            .check()?;
            let primary: Primary = *lib.get(b"heif_context_get_primary_image_handle\0")?;
            let mut handle = Guard {
                ptr: std::ptr::null_mut(),
                release: *lib.get(b"heif_image_handle_release\0")?,
            };
            primary(context.ptr, &mut handle.ptr).check()?;
            let width: Dimension = *lib.get(b"heif_image_handle_get_width\0")?;
            let height: Dimension = *lib.get(b"heif_image_handle_get_height\0")?;
            let (w, h) = (width(handle.ptr), height(handle.ptr));
            anyhow::ensure!(
                w > 0
                    && h > 0
                    && w <= 30000
                    && h <= 30000
                    && (w as u64) * (h as u64) <= 100_000_000,
                "HEIC image exceeds the 100 megapixel limit"
            );
            let decode: Decode = *lib.get(b"heif_decode_image\0")?;
            let mut decoded = Guard {
                ptr: std::ptr::null_mut(),
                release: *lib.get(b"heif_image_release\0")?,
            };
            // RGB color space, interleaved RGBA; libheif applies HEIF transforms.
            decode(handle.ptr, &mut decoded.ptr, 1, 11, std::ptr::null()).check()?;
            let get_plane: Plane = *lib.get(b"heif_image_get_plane_readonly\0")?;
            let mut stride = 0;
            let data = get_plane(decoded.ptr, 10, &mut stride);
            anyhow::ensure!(
                !data.is_null() && stride >= w * 4,
                "Invalid HEIC pixel plane"
            );
            let mut rgb = RgbImage::new(w as u32, h as u32);
            for y in 0..h as usize {
                let row = std::slice::from_raw_parts(data.add(y * stride as usize), w as usize * 4);
                for x in 0..w as usize {
                    let p = &row[x * 4..x * 4 + 4];
                    let a = p[3] as u32;
                    rgb.put_pixel(
                        x as u32,
                        y as u32,
                        image::Rgb(std::array::from_fn(|i| {
                            ((p[i] as u32 * a + 255 * (255 - a) + 127) / 255) as u8
                        })),
                    );
                }
            }
            let profile_size: ProfileSize =
                *lib.get(b"heif_image_handle_get_raw_color_profile_size\0")?;
            let len = profile_size(handle.ptr);
            let profile = if len > 0 && len <= 16 * 1024 * 1024 {
                let mut profile = vec![0; len];
                let get: Profile = *lib.get(b"heif_image_handle_get_raw_color_profile\0")?;
                get(handle.ptr, profile.as_mut_ptr().cast()).check()?;
                Some(profile)
            } else {
                None
            };
            Ok((rgb, profile))
        }
    }

    fn open_library(path: &Path) -> std::result::Result<Library, libloading::Error> {
        #[cfg(windows)]
        unsafe {
            // Include the DLL's own directory when resolving libde265 and the CRT.
            let flags = 0x1000 | if path.is_absolute() { 0x100 } else { 0 };
            libloading::os::windows::Library::load_with_flags(path, flags).map(Into::into)
        }
        #[cfg(not(windows))]
        unsafe {
            Library::new(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;
    #[test]
    fn png_alpha_is_flattened_and_corrupt_input_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("photo.png");
        image::RgbaImage::from_pixel(4, 3, image::Rgba([0, 0, 0, 0]))
            .save(&path)
            .unwrap();
        let source = load(&path).unwrap();
        assert_eq!(source.pixels.dimensions(), (4, 3));
        assert_eq!(source.pixels.get_pixel(0, 0).0, [255, 255, 255]);
        std::fs::write(&path, b"invalid").unwrap();
        assert!(load(&path).is_err());
    }
    #[test]
    fn jpeg_exif_orientation_is_applied_before_sizing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oriented.jpg");
        // Little-endian TIFF with one IFD entry: Orientation=6 (90 degrees CW).
        let exif = vec![
            b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
        ];
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 100);
        encoder.set_exif_metadata(exif).unwrap();
        encoder
            .encode_image(&RgbImage::from_pixel(40, 20, image::Rgb([180, 30, 70])))
            .unwrap();
        assert_eq!(load(&path).unwrap().pixels.dimensions(), (20, 40));
    }
    #[test]
    fn embedded_rgb_profile_is_converted_to_srgb() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profile.png");
        let srgb = lcms2::Profile::new_srgb();
        let profile = srgb.icc().unwrap();
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = image::codecs::png::PngEncoder::new(file);
        encoder.set_icc_profile(profile).unwrap();
        encoder
            .write_image(&[40, 90, 180], 1, 1, image::ExtendedColorType::Rgb8)
            .unwrap();
        let source = load(&path).unwrap();
        for (actual, expected) in source
            .pixels
            .get_pixel(0, 0)
            .0
            .iter()
            .zip([40_i16, 90, 180])
        {
            assert!((*actual as i16 - expected).abs() <= 1);
        }
    }
    #[test]
    fn bundled_heic_runtime_decodes_a_real_primary_image_when_fixture_is_provided() {
        // CI always supplies the official libheif example; local dependency-free
        // test runs can omit it. Supplying the fixture makes decoder failures fatal.
        let Some(path) = std::env::var_os("PRINTMARTEN_HEIC_FIXTURE") else {
            return;
        };
        let source = load(Path::new(&path)).unwrap();
        assert!(source.pixels.width() > 100 && source.pixels.height() > 100);
        assert!(source.pixels.pixels().any(|p| p.0 != [255, 255, 255]));
    }
}
