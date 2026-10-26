/// Shared artwork for the native window icon and the application toolbar.
pub fn icon(size: u32) -> image::RgbaImage {
    image::load_from_memory(include_bytes!("../icon.png"))
        .expect("The embedded PrintMarten icon must be a valid PNG")
        .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
        .to_rgba8()
}
