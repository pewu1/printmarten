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
    fn contains(self, b: Self) -> bool {
        self.x <= b.x
            && self.y <= b.y
            && self.x + self.w >= b.x + b.w
            && self.y + self.h >= b.y + b.h
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

/// Split every intersected free rectangle, then remove contained duplicates (MaxRects).
fn occupy(free: &mut Vec<Rect>, used: Rect) {
    let mut next = Vec::new();
    for r in free.drain(..) {
        if !r.intersects(used) {
            next.push(r);
            continue;
        }
        if used.x > r.x {
            next.push(Rect {
                w: used.x - r.x,
                ..r
            });
        }
        if used.x + used.w < r.x + r.w {
            next.push(Rect {
                x: used.x + used.w,
                w: r.x + r.w - used.x - used.w,
                ..r
            });
        }
        if used.y > r.y {
            next.push(Rect {
                h: used.y - r.y,
                ..r
            });
        }
        if used.y + used.h < r.y + r.h {
            next.push(Rect {
                y: used.y + used.h,
                h: r.y + r.h - used.y - used.h,
                ..r
            });
        }
    }
    let mut i = 0;
    while i < next.len() {
        if (0..next.len())
            .any(|j| i != j && next[j].contains(next[i]) && (next[j] != next[i] || j < i))
        {
            next.remove(i);
        } else {
            i += 1;
        }
    }
    *free = next;
}

pub fn arrange(photos: &[Photo], settings: &Settings, hardware: Insets) -> Layout {
    let margins = hardware.max(Insets::uniform(settings.layout_margin()));
    let area = Rect {
        x: margins.left,
        y: margins.top,
        w: settings.paper.w - margins.left - margins.right,
        h: settings.paper.h - margins.top - margins.bottom,
    };
    if settings.full_page {
        return Layout {
            paper: settings.paper,
            pages: if settings.paper.valid() && area.w > 0 && area.h > 0 {
                photos
                    .iter()
                    .map(|photo| Page {
                        photos: vec![Placement {
                            photo_id: photo.id,
                            source: photo.source,
                            rect: area,
                            rotated: false,
                            fit: Fit::Fill,
                        }],
                    })
                    .collect()
            } else {
                vec![]
            },
            rejected: if settings.paper.valid() && area.w > 0 && area.h > 0 {
                vec![]
            } else {
                photos.iter().map(|photo| photo.id).collect()
            },
            cut_lines: false,
        };
    }
    let mut best: Option<Layout> = None;
    // Different sorts and scoring rules improve mixed-size layouts without unstable randomization.
    for sort in 0..3 {
        for score_kind in 0..2 {
            let mut ordered: Vec<_> = photos.iter().collect();
            ordered.sort_by_key(|p| {
                let s = p.size(settings.default_size);
                let key = match sort {
                    0 => s.w * s.h,
                    1 => s.w.max(s.h),
                    _ => s.w.min(s.h),
                };
                (std::cmp::Reverse(key), p.id)
            });
            let gap = settings.gap.max(0);
            // Include one trailing gap in the packing bin so photos can touch the usable edge.
            let bin = Rect {
                w: area.w + gap,
                h: area.h + gap,
                ..area
            };
            let mut free_pages: Vec<Vec<Rect>> = Vec::new();
            let mut result = Layout {
                paper: settings.paper,
                pages: vec![],
                rejected: vec![],
                cut_lines: settings.cut_lines,
            };
            for p in ordered {
                let s = p.size(settings.default_size);
                if !settings.paper.valid()
                    || !s.valid()
                    || area.w <= 0
                    || area.h <= 0
                    || !((s.w <= area.w && s.h <= area.h) || (s.h <= area.w && s.w <= area.h))
                {
                    result.rejected.push(p.id);
                    continue;
                }
                let mut choice = None;
                for page in 0..=free_pages.len() {
                    let candidates: &[Rect] = if page == free_pages.len() {
                        std::slice::from_ref(&bin)
                    } else {
                        &free_pages[page]
                    };
                    for &r in candidates {
                        for rotated in [false, true] {
                            let size = if rotated { s.rotated() } else { s };
                            let (w, h) = (size.w + gap, size.h + gap);
                            if w > r.w || h > r.h {
                                continue;
                            }
                            let short = (r.w - w).min(r.h - h);
                            let waste = r.w * r.h - w * h;
                            let score = if score_kind == 0 {
                                (short, waste)
                            } else {
                                (waste, short)
                            };
                            let rank = (page, score, r.y, r.x, rotated);
                            if choice.as_ref().is_none_or(|(old, _, _)| rank < *old) {
                                choice = Some((
                                    rank,
                                    Rect {
                                        x: r.x,
                                        y: r.y,
                                        w,
                                        h,
                                    },
                                    rotated,
                                ));
                            }
                        }
                    }
                }
                let (rank, used, rotated) = choice.expect("validated photo must fit a fresh bin");
                let page = rank.0;
                if page == free_pages.len() {
                    free_pages.push(vec![bin]);
                    result.pages.push(Page::default());
                }
                occupy(&mut free_pages[page], used);
                result.pages[page].photos.push(Placement {
                    photo_id: p.id,
                    source: p.source,
                    rect: Rect {
                        w: used.w - gap,
                        h: used.h - gap,
                        ..used
                    },
                    rotated,
                    fit: p.fit,
                });
            }
            result.rejected.sort_unstable();
            if best.as_ref().is_none_or(|b| quality(&result) < quality(b)) {
                best = Some(result);
            }
        }
    }
    best.unwrap()
}

fn quality(layout: &Layout) -> (usize, i64) {
    (
        layout.pages.len(),
        layout
            .pages
            .iter()
            .map(|p| {
                p.photos
                    .iter()
                    .map(|p| p.rect.y + p.rect.h)
                    .max()
                    .unwrap_or(0)
            })
            .sum(),
    )
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
                override_size: Some(Size::mm(30, 40)),
                fit: Fit::Contain,
            },
            Photo {
                id: 2,
                source: 0,
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
        assert_eq!(
            arrange(&photos(2, None), &settings, Insets::default())
                .pages
                .len(),
            1
        );
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
