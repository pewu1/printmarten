use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: Length,
    pub y: Length,
    pub w: Length,
    pub h: Length,
}
impl Rect {
    pub fn intersects(self, b: Self) -> bool {
        self.x < b.x + b.w && self.x + self.w > b.x && self.y < b.y + b.h && self.y + self.h > b.y
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub photo_id: u64,
    pub source: usize,
    pub rect: Rect,
    pub rotated: bool,
    pub fit: Fit,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub photos: Vec<Placement>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    pub paper: Size,
    pub pages: Vec<Page>,
    pub rejected: Vec<u64>,
    pub cut_lines: bool,
}

/// Rotate physical photo dimensions so their long edge follows the sheet.
fn sheet_size(size: Size, paper: Size) -> Size {
    if size.w != size.h && paper.w != paper.h && (size.w > size.h) != (paper.w > paper.h) {
        size.rotated()
    } else {
        size
    }
}

fn rotate_image(photo: &Photo, target: Size) -> bool {
    target.w != target.h && photo.source_landscape != (target.w > target.h)
}

#[derive(Default)]
struct Row {
    photos: Vec<Placement>,
    width: Length,
    height: Length,
}
#[derive(Default)]
struct Sheet {
    rows: Vec<Row>,
    height: Length,
}

/// Fixed-size photos in balanced rows, with equal extra space on every sheet.
pub fn arrange(photos: &[Photo], settings: &Settings, hardware: Insets) -> Layout {
    let margins = hardware.max(Insets::uniform(settings.layout_margin()));
    let area = Rect {
        x: margins.left,
        y: margins.top,
        w: settings.paper.w - margins.left - margins.right,
        h: settings.paper.h - margins.top - margins.bottom,
    };
    let valid = settings.paper.valid() && area.w > 0 && area.h > 0;
    if settings.full_page {
        let target = Size {
            w: area.w,
            h: area.h,
        };
        return Layout {
            paper: settings.paper,
            pages: if valid {
                photos
                    .iter()
                    .map(|photo| Page {
                        photos: vec![Placement {
                            photo_id: photo.id,
                            source: photo.source,
                            rect: area,
                            rotated: rotate_image(photo, target),
                            fit: Fit::Fill,
                        }],
                    })
                    .collect()
            } else {
                vec![]
            },
            rejected: if valid {
                vec![]
            } else {
                photos.iter().map(|photo| photo.id).collect()
            },
            cut_lines: false,
        };
    }
    let gap = settings.gap.max(0);
    let mut best = None;
    // Try several deterministic orders to reduce sheet count for mixed sizes.
    for sort in 0..3 {
        let mut ordered: Vec<_> = photos.iter().collect();
        ordered.sort_by_key(|photo| {
            let size = sheet_size(photo.size(settings.default_size), settings.paper);
            let score = match sort {
                0 => (size.h, size.w),
                1 => (size.w * size.h, size.h),
                _ => (size.w, size.h),
            };
            (std::cmp::Reverse(score), photo.id)
        });
        let mut sheets: Vec<Sheet> = vec![];
        let mut rejected = vec![];
        for photo in ordered {
            let size = sheet_size(photo.size(settings.default_size), settings.paper);
            if !valid || !size.valid() || size.w > area.w || size.h > area.h {
                rejected.push(photo.id);
                continue;
            }
            let mut choice = None;
            for (page, sheet) in sheets.iter().enumerate() {
                for (row_index, row) in sheet.rows.iter().enumerate() {
                    let extra_height = (size.h - row.height).max(0);
                    let width = row.width + gap + size.w;
                    if width <= area.w && sheet.height + extra_height <= area.h {
                        let rank = (page, extra_height, area.w - width, row_index);
                        if choice.is_none_or(|old| rank < old) {
                            choice = Some(rank);
                        }
                    }
                }
                if choice.is_some_and(|rank| rank.0 == page) {
                    break;
                }
                if sheet.height + gap + size.h <= area.h {
                    choice = Some((page, size.h + gap, area.w - size.w, sheet.rows.len()));
                    break;
                }
            }
            let (page, _, _, row_index) = choice.unwrap_or((sheets.len(), 0, 0, 0));
            if page == sheets.len() {
                sheets.push(Sheet::default());
            }
            let sheet = &mut sheets[page];
            if row_index == sheet.rows.len() {
                if !sheet.rows.is_empty() {
                    sheet.height += gap;
                }
                sheet.rows.push(Row::default());
            }
            let row = &mut sheet.rows[row_index];
            sheet.height += (size.h - row.height).max(0);
            row.height = row.height.max(size.h);
            if !row.photos.is_empty() {
                row.width += gap;
            }
            row.width += size.w;
            row.photos.push(Placement {
                photo_id: photo.id,
                source: photo.source,
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: size.w,
                    h: size.h,
                },
                rotated: rotate_image(photo, size),
                fit: photo.fit,
            });
        }
        let score = (
            sheets.len(),
            sheets.iter().map(|sheet| sheet.height).sum::<Length>(),
        );
        if best
            .as_ref()
            .is_none_or(|(old_score, _, _)| score < *old_score)
        {
            best = Some((score, sheets, rejected));
        }
    }
    let (_, sheets, mut rejected) = best.unwrap();
    rejected.sort_unstable();
    let pages = sheets
        .into_iter()
        .map(|sheet| {
            let extra_y = area.h - sheet.height;
            let row_count = sheet.rows.len() as Length;
            let mut y = area.y;
            let mut page = Page::default();
            for (row_index, row) in sheet.rows.into_iter().enumerate() {
                let top = y + extra_y * (row_index as Length + 1) / (row_count + 1);
                let extra_x = area.w - row.width;
                let count = row.photos.len() as Length;
                let mut x = area.x;
                for (column, mut photo) in row.photos.into_iter().enumerate() {
                    photo.rect.x = x + extra_x * (column as Length + 1) / (count + 1);
                    photo.rect.y = top + (row.height - photo.rect.h) / 2;
                    x += photo.rect.w + gap;
                    page.photos.push(photo);
                }
                y += row.height + gap;
            }
            page
        })
        .collect();
    Layout {
        paper: settings.paper,
        pages,
        rejected,
        cut_lines: settings.cut_lines,
    }
}

/// Normalized UV rectangle; rotation is applied to the image before this crop.
pub fn image_geometry(image: (u32, u32), target: (f64, f64), fit: Fit) -> ([f32; 4], [f32; 4]) {
    let source_ratio = image.0 as f64 / image.1 as f64;
    let target_ratio = target.0 / target.1;
    let mut uv = [0.0, 0.0, 1.0, 1.0];
    let mut dest = [0.0, 0.0, 1.0, 1.0];
    if fit == Fit::Fill {
        if source_ratio > target_ratio {
            let w = (target_ratio / source_ratio) as f32;
            uv[0] = (1.0 - w) / 2.0;
            uv[2] = uv[0] + w;
        } else {
            let h = (source_ratio / target_ratio) as f32;
            uv[1] = (1.0 - h) / 2.0;
            uv[3] = uv[1] + h;
        }
    } else if source_ratio > target_ratio {
        let h = (target_ratio / source_ratio) as f32;
        dest[1] = (1.0 - h) / 2.0;
        dest[3] = dest[1] + h;
    } else {
        let w = (source_ratio / target_ratio) as f32;
        dest[0] = (1.0 - w) / 2.0;
        dest[2] = dest[0] + w;
    }
    (uv, dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_page_crops_each_photo_to_the_sheet_and_keeps_custom_sizes() {
        let photos = vec![
            Photo {
                id: 1,
                source: 0,
                source_landscape: false,
                override_size: Some(Size::mm(30, 40)),
                fit: Fit::Contain,
            },
            Photo {
                id: 2,
                source: 0,
                source_landscape: false,
                override_size: None,
                fit: Fit::Fill,
            },
        ];
        for paper in [Size::mm(100, 150), Size::mm(150, 100)] {
            let mut settings = Settings {
                paper,
                full_page: true,
                borderless: true,
                cut_lines: true,
                ..Settings::default()
            };
            let layout = arrange(&photos, &settings, Insets::default());
            assert_eq!(layout.pages.len(), 2);
            assert!(layout.rejected.is_empty());
            assert!(!layout.cut_lines);
            for (page, photo) in layout.pages.iter().zip(&photos) {
                assert_eq!(page.photos.len(), 1);
                assert_eq!(page.photos[0].photo_id, photo.id);
                assert_eq!(
                    page.photos[0].rect,
                    Rect {
                        x: 0,
                        y: 0,
                        w: paper.w,
                        h: paper.h
                    }
                );
                assert_eq!(page.photos[0].fit, Fit::Fill);
            }
            settings.full_page = false;
            let packed = arrange(&photos, &settings, Insets::default());
            let customized = packed
                .pages
                .iter()
                .flat_map(|page| &page.photos)
                .find(|p| p.photo_id == 1)
                .unwrap();
            assert_eq!(customized.rect.w * customized.rect.h, 30000 * 40000);
            assert_eq!(customized.fit, Fit::Contain);
            settings.full_page = true;
            settings.borderless = false;
            let bordered = arrange(&photos, &settings, Insets::uniform(7000));
            assert_eq!(
                bordered.pages[0].photos[0].rect,
                Rect {
                    x: 7000,
                    y: 7000,
                    w: paper.w - 14000,
                    h: paper.h - 14000
                }
            );
        }
    }
    fn photos(n: u64, size: Option<Size>) -> Vec<Photo> {
        (0..n)
            .map(|id| Photo {
                id,
                source: 0,
                source_landscape: false,
                override_size: size,
                fit: Fit::Fill,
            })
            .collect()
    }
    #[test]
    fn six_small_photos_fit_and_overflow_creates_next_sheet() {
        let mut settings = Settings {
            default_size: Size::mm(60, 90),
            ..Settings::default()
        };
        assert_eq!(
            arrange(&photos(6, None), &settings, Insets::default())
                .pages
                .len(),
            1
        );
        assert_eq!(
            arrange(&photos(10, None), &settings, Insets::default())
                .pages
                .len(),
            2
        );
        settings.default_size = Size::mm(100, 150);
        // Long edges stay parallel to the portrait sheet, even when rotating
        // a rectangle would allow a more compact arrangement.
        assert_eq!(
            arrange(&photos(2, None), &settings, Insets::default())
                .pages
                .len(),
            2
        );
    }
    #[test]
    fn image_and_print_dimensions_follow_the_long_sheet_edge() {
        for paper in [Size::mm(210, 297), Size::mm(297, 210)] {
            for source_landscape in [false, true] {
                for size in [Size::mm(60, 90), Size::mm(90, 60)] {
                    let mut settings = Settings {
                        paper,
                        default_size: size,
                        ..Settings::default()
                    };
                    let mut photos = photos(3, None);
                    for photo in &mut photos {
                        photo.source_landscape = source_landscape;
                    }
                    for full_page in [false, true] {
                        settings.full_page = full_page;
                        let layout = arrange(&photos, &settings, Insets::default());
                        for placement in layout.pages.iter().flat_map(|page| &page.photos) {
                            assert_eq!(placement.rect.w > placement.rect.h, paper.w > paper.h);
                            assert_eq!(placement.rotated, source_landscape != (paper.w > paper.h));
                            if !full_page {
                                assert_eq!(placement.rect.w * placement.rect.h, size.w * size.h);
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn equal_size_rows_have_equal_gaps_and_balanced_outer_spacing() {
        let settings = Settings {
            default_size: Size::mm(60, 90),
            ..Settings::default()
        };
        let layout = arrange(&photos(6, None), &settings, Insets::default());
        let placements = &layout.pages[0].photos;
        assert_eq!(placements.len(), 6);
        let margin = settings.layout_margin();
        let first = &placements[..3];
        let second = &placements[3..];
        for row in [first, second] {
            let extras = [
                row[0].rect.x - margin,
                row[1].rect.x - row[0].rect.x - row[0].rect.w - settings.gap,
                row[2].rect.x - row[1].rect.x - row[1].rect.w - settings.gap,
                settings.paper.w - margin - row[2].rect.x - row[2].rect.w,
            ];
            assert!(extras.iter().max().unwrap() - extras.iter().min().unwrap() <= 1);
        }
        let extras = [
            first[0].rect.y - margin,
            second[0].rect.y - first[0].rect.y - first[0].rect.h - settings.gap,
            settings.paper.h - margin - second[0].rect.y - second[0].rect.h,
        ];
        assert!(extras.iter().max().unwrap() - extras.iter().min().unwrap() <= 1);
        let partial = arrange(&photos(4, None), &settings, Insets::default());
        let last = partial.pages[0].photos.last().unwrap();
        assert_eq!(last.rect.x * 2 + last.rect.w, settings.paper.w);
    }
    #[test]
    fn mixed_layout_is_deterministic_and_respects_hardware_and_gap() {
        let settings = Settings::default();
        let mut input = photos(30, None);
        for p in &mut input {
            p.override_size = Some(Size::mm(
                20 + (p.id as i64 * 17) % 100,
                30 + (p.id as i64 * 23) % 100,
            ));
        }
        let hardware = Insets {
            left: 12000,
            right: 7000,
            top: 8000,
            bottom: 11000,
        };
        let result = arrange(&input, &settings, hardware);
        assert_eq!(result, arrange(&input, &settings, hardware));
        assert!(result.rejected.is_empty());
        assert_eq!(
            result.pages.iter().map(|p| p.photos.len()).sum::<usize>(),
            30
        );
        for page in result.pages {
            for (i, p) in page.photos.iter().enumerate() {
                assert!(p.rect.x >= hardware.left && p.rect.y >= hardware.top);
                assert!(p.rect.x + p.rect.w <= settings.paper.w - hardware.right);
                assert!(p.rect.y + p.rect.h <= settings.paper.h - hardware.bottom);
                for q in page.photos.iter().skip(i + 1) {
                    assert!(
                        !Rect {
                            w: p.rect.w + settings.gap,
                            h: p.rect.h + settings.gap,
                            ..p.rect
                        }
                        .intersects(q.rect)
                    );
                }
            }
        }
    }
    #[test]
    fn rejects_oversized_and_handles_zero_gap_and_exact_fit() {
        let settings = Settings {
            paper: Size::mm(100, 150),
            margin: 0,
            gap: 0,
            ..Settings::default()
        };
        assert_eq!(
            arrange(&photos(2, None), &settings, Insets::default())
                .pages
                .len(),
            2
        );
        assert_eq!(
            arrange(
                &photos(1, Some(Size::mm(101, 151))),
                &settings,
                Insets::default()
            )
            .rejected,
            vec![0]
        );
    }
    #[test]
    fn crop_and_contain_do_not_distort() {
        let (uv, dest) = image_geometry((400, 200), (100.0, 100.0), Fit::Fill);
        assert_eq!(uv, [0.25, 0.0, 0.75, 1.0]);
        assert_eq!(dest, [0.0, 0.0, 1.0, 1.0]);
        let (uv, dest) = image_geometry((400, 200), (100.0, 100.0), Fit::Contain);
        assert_eq!(uv, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(dest, [0.0, 0.25, 1.0, 0.75]);
    }
}
