use crate::{
    imaging::Source,
    layout::{Layout, Placement, image_geometry},
    model::*,
};
use anyhow::{Context, Result};
use image::{Rgb, RgbImage};
use lopdf::{
    Document, Object, Stream,
    content::{Content, Operation},
    dictionary,
};
use std::{collections::HashMap, path::Path};

pub fn oriented(source: &Source, rotated: bool) -> RgbImage {
    if rotated {
        image::imageops::rotate90(source.pixels.as_ref())
    } else {
        source.pixels.as_ref().clone()
    }
}

/// Full-resolution image XObjects plus physical PDF transforms and vector cut lines.
pub fn write_pdf(layout: &Layout, sources: &[Source], path: &Path) -> Result<()> {
    anyhow::ensure!(
        layout.rejected.is_empty() && !layout.pages.is_empty(),
        "The layout is not printable"
    );
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut images = HashMap::new();
    let mut kids = Vec::new();
    let pt = |v: Length| v as f32 * 72.0 / 25400.0;
    for page in &layout.pages {
        let mut ops = Vec::new();
        let mut xobjects = lopdf::Dictionary::new();
        for p in &page.photos {
            let source = sources.get(p.source).context("Missing image source")?;
            let key = (p.source, p.rotated);
            let id = if let Some(id) = images.get(&key) {
                *id
            } else {
                let pixels = oriented(source, p.rotated);
                let mut jpeg = Vec::new();
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 95)
                    .encode_image(&pixels)?;
                let stream = Stream::new(
                    dictionary! { "Type"=>"XObject", "Subtype"=>"Image", "Width"=>pixels.width(), "Height"=>pixels.height(), "ColorSpace"=>"DeviceRGB", "BitsPerComponent"=>8, "Filter"=>"DCTDecode" },
                    jpeg,
                );
                let id = doc.add_object(stream);
                images.insert(key, id);
                id
            };
            let name = format!("Im{}_{}", p.source, u8::from(p.rotated));
            xobjects.set(name.as_str(), id);
            let dims = source.pixels.dimensions();
            let dims = if p.rotated { (dims.1, dims.0) } else { dims };
            let (uv, dest) = image_geometry(dims, (p.rect.w as f64, p.rect.h as f64), p.fit);
            let (x, y, w, h) = (
                pt(p.rect.x),
                pt(layout.paper.h - p.rect.y - p.rect.h),
                pt(p.rect.w),
                pt(p.rect.h),
            );
            let iw = w * (dest[2] - dest[0]) / (uv[2] - uv[0]);
            let ih = h * (dest[3] - dest[1]) / (uv[3] - uv[1]);
            let ix = x + w * dest[0] - iw * uv[0];
            let iy = y + h * (1.0 - dest[3]) - ih * (1.0 - uv[3]);
            ops.push(Operation::new("q", vec![]));
            ops.push(Operation::new(
                "re",
                vec![x.into(), y.into(), w.into(), h.into()],
            ));
            ops.push(Operation::new("W", vec![]));
            ops.push(Operation::new("n", vec![]));
            ops.push(Operation::new(
                "cm",
                vec![
                    iw.into(),
                    0.into(),
                    0.into(),
                    ih.into(),
                    ix.into(),
                    iy.into(),
                ],
            ));
            ops.push(Operation::new("Do", vec![Object::Name(name.into_bytes())]));
            ops.push(Operation::new("Q", vec![]));
            if layout.cut_lines {
                ops.push(Operation::new("q", vec![]));
                ops.push(Operation::new("w", vec![0.25.into()]));
                ops.push(Operation::new("G", vec![0.into()]));
                ops.push(Operation::new(
                    "re",
                    vec![x.into(), y.into(), w.into(), h.into()],
                ));
                ops.push(Operation::new("S", vec![]));
                ops.push(Operation::new("Q", vec![]));
            }
        }
        let content = doc.add_object(Stream::new(
            dictionary! {},
            Content { operations: ops }.encode()?,
        ));
        let id=doc.add_object(dictionary! { "Type"=>"Page", "Parent"=>pages_id,
            "MediaBox"=>vec![0.into(),0.into(),Object::Real(pt(layout.paper.w)),Object::Real(pt(layout.paper.h))],
            "Resources"=>dictionary! { "XObject"=>xobjects },"Contents"=>content });
        kids.push(Object::Reference(id));
    }
    doc.objects.insert(
        pages_id,
        dictionary! { "Type"=>"Pages", "Count"=>kids.len() as i64,"Kids"=>kids }.into(),
    );
    let catalog = doc.add_object(dictionary! { "Type"=>"Catalog","Pages"=>pages_id });
    doc.trailer.set("Root", catalog);
    doc.compress();
    doc.save(path)?;
    Ok(())
}

/// Windows GDI consumes a page at the printer's resolution, bounded to 300 DPI.
/// Preview and PDF rendering use the same crop/contain geometry.
pub fn raster_page(
    layout: &Layout,
    index: usize,
    sources: &[Source],
    dpi: f64,
) -> Result<RgbImage> {
    let scale = dpi / 25400.0;
    let width = (layout.paper.w as f64 * scale).round() as u32;
    let height = (layout.paper.h as f64 * scale).round() as u32;
    anyhow::ensure!(
        width > 0 && height > 0 && width as u64 * height as u64 <= 150_000_000,
        "Page is too large to render"
    );
    let mut page = RgbImage::from_pixel(width, height, Rgb([255, 255, 255]));
    for p in &layout.pages[index].photos {
        let source = sources.get(p.source).context("Missing image source")?;
        paint_photo(&mut page, source, p, scale);
        if layout.cut_lines {
            let x = (p.rect.x as f64 * scale).round() as u32;
            let y = (p.rect.y as f64 * scale).round() as u32;
            let right = ((p.rect.x + p.rect.w) as f64 * scale).round() as u32;
            let bottom = ((p.rect.y + p.rect.h) as f64 * scale).round() as u32;
            let stroke = (dpi * 0.25 / 72.0).round().max(1.0) as u32;
            for t in 0..stroke {
                for xx in x..=right.min(width - 1) {
                    for yy in [y.saturating_add(t), bottom.saturating_sub(t)] {
                        if yy < height {
                            page.put_pixel(xx, yy, Rgb([0, 0, 0]));
                        }
                    }
                }
                for yy in y..=bottom.min(height - 1) {
                    for xx in [x.saturating_add(t), right.saturating_sub(t)] {
                        if xx < width {
                            page.put_pixel(xx, yy, Rgb([0, 0, 0]));
                        }
                    }
                }
            }
        }
    }
    Ok(page)
}
fn paint_photo(page: &mut RgbImage, source: &Source, p: &Placement, scale: f64) {
    let image = oriented(source, p.rotated);
    let (uv, dest) = image_geometry(
        image.dimensions(),
        (p.rect.w as f64, p.rect.h as f64),
        p.fit,
    );
    let x0 = (uv[0] * image.width() as f32).round() as u32;
    let y0 = (uv[1] * image.height() as f32).round() as u32;
    let w = ((uv[2] - uv[0]) * image.width() as f32).round().max(1.0) as u32;
    let h = ((uv[3] - uv[1]) * image.height() as f32).round().max(1.0) as u32;
    let crop = image::imageops::crop_imm(
        &image,
        x0.min(image.width() - 1),
        y0.min(image.height() - 1),
        w.min(image.width() - x0.min(image.width() - 1)),
        h.min(image.height() - y0.min(image.height() - 1)),
    )
    .to_image();
    let w = (p.rect.w as f64 * scale * (dest[2] - dest[0]) as f64)
        .round()
        .max(1.0) as u32;
    let h = (p.rect.h as f64 * scale * (dest[3] - dest[1]) as f64)
        .round()
        .max(1.0) as u32;
    let resized = image::imageops::resize(&crop, w, h, image::imageops::FilterType::Lanczos3);
    let x = ((p.rect.x as f64 + p.rect.w as f64 * dest[0] as f64) * scale).round() as i64;
    let y = ((p.rect.y as f64 + p.rect.h as f64 * dest[1] as f64) * scale).round() as i64;
    image::imageops::overlay(page, &resized, x, y);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raster_places_crop_and_contain_at_physical_coordinates() {
        let mut pixels = RgbImage::from_pixel(4, 4, Rgb([255, 0, 0]));
        for y in 2..4 {
            for x in 0..4 {
                pixels.put_pixel(x, y, Rgb([0, 0, 255]));
            }
        }
        let sources = [Source {
            path: "colors".into(),
            pixels: std::sync::Arc::new(pixels),
        }];
        let mut placement = Placement {
            photo_id: 1,
            source: 0,
            rect: crate::layout::Rect {
                x: 5000,
                y: 5000,
                w: 20000,
                h: 10000,
            },
            rotated: false,
            fit: Fit::Fill,
        };
        let mut layout = Layout {
            paper: Size::mm(40, 40),
            pages: vec![crate::layout::Page {
                photos: vec![placement.clone()],
            }],
            rejected: vec![],
            cut_lines: false,
        };
        let image = raster_page(&layout, 0, &sources, 25.4).unwrap();
        assert_eq!(image.dimensions(), (40, 40));
        assert_eq!(image.get_pixel(4, 5).0, [255, 255, 255]);
        assert!(image.get_pixel(15, 5)[0] > 200);
        assert!(image.get_pixel(15, 14)[2] > 200);
        placement.fit = Fit::Contain;
        layout.pages[0].photos[0] = placement;
        let image = raster_page(&layout, 0, &sources, 25.4).unwrap();
        assert_eq!(image.get_pixel(6, 7).0, [255, 255, 255]);
        assert!(image.get_pixel(15, 7)[0] > 200);
    }
    #[test]
    fn pdf_preserves_sheet_and_photo_measurements_and_vector_cut_lines() {
        let settings = Settings {
            cut_lines: true,
            ..Settings::default()
        };
        let layout = crate::layout::arrange(
            &[Photo {
                id: 1,
                source: 0,
                override_size: None,
                fit: Fit::Fill,
            }],
            &settings,
            Insets::default(),
        );
        let source = Source {
            path: "test".into(),
            pixels: std::sync::Arc::new(RgbImage::from_pixel(30, 20, Rgb([255, 0, 0]))),
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.pdf");
        write_pdf(&layout, &[source], &path).unwrap();
        let doc = Document::load(&path).unwrap();
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 1);
        let page = doc.get_dictionary(*pages.values().next().unwrap()).unwrap();
        let media = page.get(b"MediaBox").unwrap().as_array().unwrap();
        assert!((media[2].as_float().unwrap() - 595.2756).abs() < 0.01);
        let content = doc
            .get_page_content(*pages.values().next().unwrap())
            .unwrap();
        let content = Content::decode(&content).unwrap();
        assert!(content.operations.iter().any(|op| op.operator == "S"));
        let rect = content
            .operations
            .iter()
            .find(|op| op.operator == "re")
            .unwrap();
        let p = &layout.pages[0].photos[0];
        assert!(
            (rect.operands[2].as_float().unwrap() - p.rect.w as f32 * 72.0 / 25400.0).abs() < 0.01
        );
    }
}
