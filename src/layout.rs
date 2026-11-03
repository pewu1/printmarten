use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CutLine {
    pub start: [Length; 2],
    pub end: [Length; 2],
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub photos: Vec<Placement>,
    pub cut_lines: Vec<CutLine>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    pub paper: Size,
    pub pages: Vec<Page>,
    pub rejected: Vec<u64>,
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
fn arrange_rows(photos: &[Photo], settings: &Settings, hardware: Insets) -> Layout {
    let margins = hardware.max(Insets::uniform(settings.layout_margin()));
    let area = Rect {
        x: margins.left,
        y: margins.top,
        w: settings.paper.w - margins.left - margins.right,
        h: settings.paper.h - margins.top - margins.bottom,
    };
    let valid = settings.paper.valid() && area.w > 0 && area.h > 0;
    let mut full_pages = vec![];
    let mut full_rejected = vec![];
    let mut fixed = vec![];
    for photo in photos {
        match photo.size(settings.default_size) {
            PrintSize::FullPage if valid => full_pages.push(Page {
                photos: vec![Placement {
                    photo_id: photo.id,
                    source: photo.source,
                    rect: area,
                    rotated: rotate_image(
                        photo,
                        Size {
                            w: area.w,
                            h: area.h,
                        },
                    ),
                    fit: photo.fit,
                }],
                cut_lines: vec![],
            }),
            PrintSize::FullPage => full_rejected.push(photo.id),
            PrintSize::Fixed(size) => fixed.push((photo, sheet_size(size, settings.paper))),
        }
    }
    let gap = settings.gap.max(0);
    let mut best = None;
    // Try several deterministic orders to reduce sheet count for mixed sizes.
    for sort in 0..6 {
        let grouped = sort >= 3;
        let mut ordered = fixed.clone();
        ordered.sort_by_key(|(photo, size)| {
            let score = match sort % 3 {
                0 => (size.h, size.w),
                1 => (size.w * size.h, size.h),
                _ => (size.w, size.h),
            };
            (std::cmp::Reverse(score), photo.id)
        });
        let mut sheets: Vec<Sheet> = vec![];
        let mut rejected = vec![];
        for (photo, size) in ordered {
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
                        let mixed = grouped
                            && row
                                .photos
                                .iter()
                                .any(|p| p.rect.w != size.w || p.rect.h != size.h);
                        let rank = (page, mixed, extra_height, area.w - width, row_index);
                        if choice.is_none_or(|old| rank < old) {
                            choice = Some(rank);
                        }
                    }
                }
                if sheet.height + gap + size.h <= area.h {
                    let rank = (page, false, size.h + gap, area.w - size.w, sheet.rows.len());
                    // Legacy candidates keep their original first-fit behavior.
                    if choice.is_none() || (grouped && choice.is_some_and(|old| rank < old)) {
                        choice = Some(rank);
                    }
                }
                if choice.is_some_and(|rank| rank.0 == page) {
                    break;
                }
            }
            let (page, _, _, _, row_index) = choice.unwrap_or((sheets.len(), false, 0, 0, 0));
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
        let mixed_rows = sheets
            .iter()
            .flat_map(|sheet| &sheet.rows)
            .filter(|row| {
                let first = &row.photos[0].rect;
                row.photos
                    .iter()
                    .any(|p| (p.rect.w, p.rect.h) != (first.w, first.h))
            })
            .count();
        let group_spread: usize = sheets
            .iter()
            .map(|sheet| {
                sheet
                    .rows
                    .iter()
                    .flat_map(|row| &row.photos)
                    .map(|p| (p.rect.w, p.rect.h))
                    .collect::<BTreeSet<_>>()
                    .len()
            })
            .sum();
        let score = (
            sheets.len(),
            mixed_rows,
            group_spread,
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
    rejected.extend(full_rejected);
    rejected.sort_unstable();
    let pages = sheets.into_iter().map(|sheet| {
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
        if settings.cut_lines {
            page.cut_lines = cutting_guides(&page.photos, area);
        }
        page
    });
    full_pages.extend(pages);
    Layout {
        paper: settings.paper,
        pages: full_pages,
        rejected,
    }
}

/// Bounded, deterministic multi-start MaxRects packing with a row baseline.
pub fn arrange(photos: &[Photo], settings: &Settings, hardware: Insets) -> Layout {
    let mut best = arrange_rows(photos, settings, hardware);
    let m = hardware.max(Insets::uniform(settings.layout_margin()));
    let area = Rect {
        x: m.left,
        y: m.top,
        w: settings.paper.w - m.left - m.right,
        h: settings.paper.h - m.top - m.bottom,
    };
    if area.w <= 0 || area.h <= 0 || !settings.paper.valid() {
        return best;
    }
    let gap = settings.gap.max(0);
    let mut fixed: Vec<_> = photos
        .iter()
        .filter_map(|p| match p.size(settings.default_size) {
            PrintSize::Fixed(s) => Some((p, s)),
            _ => None,
        })
        .collect();
    let score = |l: &Layout| {
        let edges: i64 = l
            .pages
            .iter()
            .flat_map(|p| &p.photos)
            .map(|p| {
                let r = p.rect;
                (if r.x == 0 { r.h } else { 0 })
                    + (if r.y == 0 { r.w } else { 0 })
                    + (if r.x + r.w == settings.paper.w {
                        r.h
                    } else {
                        0
                    })
                    + (if r.y + r.h == settings.paper.h {
                        r.w
                    } else {
                        0
                    })
            })
            .sum();
        (
            l.rejected.len(),
            l.pages.len(),
            if settings.borderless { -edges } else { 0 },
        )
    };
    for order in 0..4 {
        fixed.sort_by_key(|(p, s)| {
            (
                std::cmp::Reverse(match order {
                    0 => s.w * s.h,
                    1 => s.w.max(s.h),
                    2 => s.w.min(s.h),
                    _ => s.w + s.h,
                }),
                p.id,
            )
        });
        for heuristic in 0..2 {
            let mut pages: Vec<Page> = Vec::new();
            let mut free: Vec<Vec<Rect>> = Vec::new();
            let mut rejected = Vec::new();
            for &(photo, size) in &fixed {
                if !size.valid()
                    || ![size, size.rotated()]
                        .iter()
                        .any(|s| s.w <= area.w && s.h <= area.h)
                {
                    rejected.push(photo.id);
                    continue;
                }
                let mut choice = None;
                let fresh = [Rect {
                    w: area.w + gap,
                    h: area.h + gap,
                    ..area
                }];
                for (page, spaces) in free
                    .iter()
                    .map(Vec::as_slice)
                    .chain(std::iter::once(fresh.as_slice()))
                    .enumerate()
                {
                    for r in spaces {
                        for s in [size, size.rotated()] {
                            let (w, h) = (s.w + gap, s.h + gap);
                            if w > r.w || h > r.h {
                                continue;
                            }
                            let rank = (
                                page,
                                if heuristic == 0 {
                                    (r.w - w).min(r.h - h)
                                } else {
                                    r.w * r.h - w * h
                                },
                                (r.w - w).max(r.h - h),
                                r.y,
                                r.x,
                                s.w,
                            );
                            if choice.as_ref().is_none_or(|(old, _)| rank < *old) {
                                choice = Some((
                                    rank,
                                    Rect {
                                        x: r.x,
                                        y: r.y,
                                        w: s.w,
                                        h: s.h,
                                    },
                                ));
                            }
                        }
                    }
                    if choice.is_some() {
                        break;
                    }
                }
                let (rank, r) = choice.unwrap();
                let page = rank.0;
                if page == pages.len() {
                    pages.push(Page::default());
                    free.push(vec![Rect {
                        w: area.w + gap,
                        h: area.h + gap,
                        ..area
                    }]);
                }
                let used = Rect {
                    w: r.w + gap,
                    h: r.h + gap,
                    ..r
                };
                let mut next = Vec::new();
                for f in free[page].drain(..) {
                    if !f.intersects(used) {
                        next.push(f);
                        continue;
                    }
                    if used.x > f.x {
                        next.push(Rect {
                            w: used.x - f.x,
                            ..f
                        });
                    }
                    if used.y > f.y {
                        next.push(Rect {
                            h: used.y - f.y,
                            ..f
                        });
                    }
                    if used.x + used.w < f.x + f.w {
                        next.push(Rect {
                            x: used.x + used.w,
                            w: f.x + f.w - used.x - used.w,
                            ..f
                        });
                    }
                    if used.y + used.h < f.y + f.h {
                        next.push(Rect {
                            y: used.y + used.h,
                            h: f.y + f.h - used.y - used.h,
                            ..f
                        });
                    }
                }
                free[page] = next
                    .iter()
                    .enumerate()
                    .filter(|(i, a)| {
                        !next.iter().enumerate().any(|(j, b)| {
                            j != *i
                                && b.x <= a.x
                                && b.y <= a.y
                                && b.x + b.w >= a.x + a.w
                                && b.y + b.h >= a.y + a.h
                                && (a != &b || j < *i)
                        })
                    })
                    .map(|(_, r)| *r)
                    .collect();
                pages[page].photos.push(Placement {
                    photo_id: photo.id,
                    source: photo.source,
                    rect: r,
                    rotated: rotate_image(photo, Size { w: r.w, h: r.h }),
                    fit: photo.fit,
                });
            }
            let mut candidate = arrange_rows(
                &photos
                    .iter()
                    .filter(|p| p.size(settings.default_size) == PrintSize::FullPage)
                    .cloned()
                    .collect::<Vec<_>>(),
                settings,
                hardware,
            );
            for page in &mut pages {
                if settings.cut_lines {
                    page.cut_lines = cutting_guides(&page.photos, area);
                }
            }
            candidate.pages.extend(pages);
            candidate.rejected.extend(rejected);
            candidate.rejected.sort_unstable();
            if score(&candidate) < score(&best) {
                best = candidate;
            }
        }
    }
    if settings.borderless {
        for page in &mut best.pages {
            page.cut_lines.retain(|l| {
                !((l.start[0] == l.end[0] && [0, settings.paper.w].contains(&l.start[0]))
                    || (l.start[1] == l.end[1] && [0, settings.paper.h].contains(&l.start[1])))
            });
        }
    }
    best
}

/// Extend edge guides by up to 3 mm without crossing another print or the printable area.
fn cutting_guides(photos: &[Placement], area: Rect) -> Vec<CutLine> {
    let mut lines = BTreeSet::new();
    for (index, photo) in photos.iter().enumerate() {
        let r = photo.rect;
        for horizontal in [true, false] {
            let (low, high, bounds_low, bounds_high, edges) = if horizontal {
                (r.x, r.x + r.w, area.x, area.x + area.w, [r.y, r.y + r.h])
            } else {
                (r.y, r.y + r.h, area.y, area.y + area.h, [r.x, r.x + r.w])
            };
            for edge in edges {
                let mut start = (low - 3000).max(bounds_low);
                let mut end = (high + 3000).min(bounds_high);
                for (other_index, other) in photos.iter().enumerate() {
                    if other_index == index {
                        continue;
                    }
                    let q = other.rect;
                    let (q_low, q_high, cross_low, cross_high) = if horizontal {
                        (q.x, q.x + q.w, q.y, q.y + q.h)
                    } else {
                        (q.y, q.y + q.h, q.x, q.x + q.w)
                    };
                    if edge >= cross_low && edge <= cross_high {
                        // Leave 0.1 mm for the guide's stroke when blank space permits.
                        if q_high <= low {
                            start = start.max((q_high + 100).min(low));
                        }
                        if q_low >= high {
                            end = end.min((q_low - 100).max(high));
                        }
                    }
                }
                lines.insert(if horizontal {
                    CutLine {
                        start: [start, edge],
                        end: [end, edge],
                    }
                } else {
                    CutLine {
                        start: [edge, start],
                        end: [edge, end],
                    }
                });
            }
        }
    }
    lines.into_iter().collect()
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
    fn two_thirteen_by_eighteen_photos_share_a4_and_touch_edges() {
        for paper in [Size::mm(210, 297), Size::mm(297, 210)] {
            let settings = Settings {
                paper,
                default_size: PrintSize::Fixed(Size::mm(130, 180)),
                ..Settings::default()
            };
            let mut input = photos(2, None);
            for p in &mut input {
                p.source_landscape = true;
            }
            let result = arrange(&input, &settings, Insets::default());
            assert_eq!(result.pages.len(), 1);
            assert_eq!(result.pages[0].photos.len(), 2);
            for p in &result.pages[0].photos {
                assert_eq!(p.rect.w * p.rect.h, 130000 * 180000);
                assert!(p.rect.x == 0 || p.rect.y == 0);
                assert_eq!(p.rotated, p.rect.w < p.rect.h);
            }
        }
    }
    #[test]
    fn mixed_full_page_and_fixed_sizes_keep_fits_and_suppress_only_full_page_guides() {
        for paper in [Size::mm(100, 150), Size::mm(150, 100)] {
            let settings = Settings {
                paper,
                cut_lines: true,
                ..Settings::default()
            };
            let mut input = photos(4, None);
            input[0].override_size = Some(PrintSize::Fixed(Size::mm(30, 40)));
            input[0].fit = Fit::Contain;
            input[1].fit = Fit::Contain;
            input[2].override_size = Some(PrintSize::FullPage);
            input[3].override_size = Some(PrintSize::Fixed(Size::mm(30, 40)));
            let layout = arrange(&input, &settings, Insets::default());
            assert_eq!(layout.pages.len(), 3);
            assert!(layout.rejected.is_empty());
            for (page, id) in layout.pages[..2].iter().zip([1, 2]) {
                assert!(page.cut_lines.is_empty());
                assert_eq!(page.photos.len(), 1);
                assert_eq!(page.photos[0].photo_id, id);
                assert_eq!(
                    page.photos[0].rect,
                    Rect {
                        x: 0,
                        y: 0,
                        w: paper.w,
                        h: paper.h
                    }
                );
            }
            assert_eq!(layout.pages[0].photos[0].fit, Fit::Contain);
            assert_eq!(layout.pages[1].photos[0].fit, Fit::Fill);
            assert_eq!(layout.pages[2].photos.len(), 2);
            assert!(!layout.pages[2].cut_lines.is_empty());
            for p in &layout.pages[2].photos {
                assert_eq!(p.rect.w * p.rect.h, 30000 * 40000);
            }
            let hardware = Insets::uniform(7000);
            let bordered = arrange(&input, &settings, hardware);
            assert_eq!(
                bordered.pages[0].photos[0].rect,
                Rect {
                    x: 7000,
                    y: 7000,
                    w: paper.w - 14000,
                    h: paper.h - 14000
                }
            );
            let invalid = arrange(&input, &settings, Insets::uniform(200000));
            assert_eq!(invalid.rejected, vec![0, 1, 2, 3]);
        }
    }

    #[test]
    fn matching_sizes_form_rows_without_wasting_an_extra_sheet() {
        let settings = Settings {
            paper: Size::mm(140, 200),
            cut_lines: true,
            default_size: PrintSize::Fixed(Size::mm(40, 60)),
            ..Settings::default()
        };
        let mut input = photos(6, None);
        for index in [1, 3, 5] {
            input[index].override_size = Some(PrintSize::Fixed(Size::mm(30, 50)));
        }
        let layout = arrange(&input, &settings, Insets::default());
        assert_eq!(layout.pages.len(), 1);
        let page = &layout.pages[0];
        assert_eq!(page.photos.len(), 6);
        let mut reversed = input.clone();
        reversed.reverse();
        assert_eq!(layout, arrange(&reversed, &settings, Insets::default()));
        // Grouping cannot force two sheets when mixing a row fits one.
        let settings = Settings {
            paper: Size::mm(80, 90),
            gap: 2000,
            ..settings
        };
        let layout = arrange(&input[..2], &settings, Insets::default());
        assert_eq!(layout.pages.len(), 1);
        assert_eq!(layout.pages[0].photos.len(), 2);
    }

    #[test]
    fn guides_extend_three_mm_and_stop_at_printable_bounds_or_other_photos() {
        let placement = |id, rect| Placement {
            photo_id: id,
            source: 0,
            rect,
            rotated: false,
            fit: Fit::Fill,
        };
        let first = placement(
            0,
            Rect {
                x: 10000,
                y: 10000,
                w: 20000,
                h: 30000,
            },
        );
        let area = Rect {
            x: 0,
            y: 0,
            w: 100000,
            h: 100000,
        };
        let guides = cutting_guides(std::slice::from_ref(&first), area);
        assert!(guides.contains(&CutLine {
            start: [7000, 10000],
            end: [33000, 10000]
        }));
        assert!(guides.contains(&CutLine {
            start: [10000, 7000],
            end: [10000, 43000]
        }));
        let neighbor = placement(
            1,
            Rect {
                x: 31000,
                y: 9000,
                w: 20000,
                h: 30000,
            },
        );
        let guides = cutting_guides(&[first.clone(), neighbor], area);
        assert!(guides.contains(&CutLine {
            start: [7000, 10000],
            end: [30900, 10000]
        }));
        let clipped = cutting_guides(
            &[first],
            Rect {
                x: 9000,
                y: 9000,
                w: 22000,
                h: 32000,
            },
        );
        assert!(clipped.contains(&CutLine {
            start: [9000, 10000],
            end: [31000, 10000]
        }));
        assert!(clipped.iter().all(|line| {
            [line.start, line.end]
                .iter()
                .all(|p| p[0] >= 9000 && p[0] <= 31000 && p[1] >= 9000 && p[1] <= 41000)
        }));
    }

    fn photos(n: u64, size: Option<Size>) -> Vec<Photo> {
        (0..n)
            .map(|id| Photo {
                id,
                source: 0,
                source_landscape: false,
                override_size: size.map(PrintSize::Fixed),
                fit: Fit::Fill,
            })
            .collect()
    }
    #[test]
    fn six_small_photos_fit_and_overflow_creates_next_sheet() {
        let mut settings = Settings {
            default_size: PrintSize::Fixed(Size::mm(60, 90)),
            borderless: false,
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
        settings.default_size = PrintSize::Fixed(Size::mm(100, 150));
        // Independent rotations avoid an unnecessary sheet.
        assert_eq!(
            arrange(&photos(2, None), &settings, Insets::default())
                .pages
                .len(),
            1
        );
    }
    #[test]
    fn image_and_print_dimensions_follow_the_long_sheet_edge() {
        for paper in [Size::mm(210, 297), Size::mm(297, 210)] {
            for source_landscape in [false, true] {
                for size in [Size::mm(60, 90), Size::mm(90, 60)] {
                    let mut settings = Settings {
                        paper,
                        default_size: PrintSize::Fixed(size),
                        ..Settings::default()
                    };
                    let mut photos = photos(3, None);
                    for photo in &mut photos {
                        photo.source_landscape = source_landscape;
                    }
                    for full_page in [false, true] {
                        settings.default_size = if full_page {
                            PrintSize::FullPage
                        } else {
                            PrintSize::Fixed(size)
                        };
                        let layout = arrange(&photos, &settings, Insets::default());
                        for placement in layout.pages.iter().flat_map(|page| &page.photos) {
                            if full_page {
                                assert_eq!(placement.rect.w > placement.rect.h, paper.w > paper.h);
                            }
                            assert_eq!(
                                placement.rotated,
                                source_landscape != (placement.rect.w > placement.rect.h)
                            );
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
            default_size: PrintSize::Fixed(Size::mm(60, 90)),
            borderless: false,
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
            p.override_size = Some(PrintSize::Fixed(Size::mm(
                20 + (p.id as i64 * 17) % 100,
                30 + (p.id as i64 * 23) % 100,
            )));
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
            default_size: PrintSize::Fixed(Size::mm(100, 150)),
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
