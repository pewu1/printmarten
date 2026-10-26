use crate::{
    imaging::Source,
    layout::{Layout, arrange},
    model::*,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Geometry {
    pub paper: Size,
    pub hardware: Insets,
    pub scale_percent: f64,
    pub pages_per_sheet: u32,
}
pub struct Prepared {
    pub geometry: Geometry,
    pub session: platform::Session,
}
pub fn prepare(settings: &Settings, pages: usize) -> Result<Option<Prepared>> {
    platform::prepare(settings, pages)
}
pub fn reconcile(photos: &[Photo], settings: &Settings, geometry: &Geometry) -> Result<Layout> {
    anyhow::ensure!(
        geometry.paper.valid(),
        "The printer returned an invalid paper size"
    );
    anyhow::ensure!(
        (geometry.scale_percent - 100.0).abs() < 0.01 && geometry.pages_per_sheet == 1,
        "Exact photo sizes require 100% scale and one page per sheet. Please change these options in the print dialog."
    );
    let mut settings = settings.clone();
    settings.paper = geometry.paper;
    if settings.borderless {
        anyhow::ensure!(
            geometry.hardware == Insets::default(),
            "The selected printer settings have unprintable margins. Choose a borderless paper size or enable borderless in the printer properties, then try again. You can also turn off Borderless in PrintMarten."
        );
    }
    let result = arrange(photos, &settings, geometry.hardware);
    anyhow::ensure!(
        result.rejected.is_empty(),
        "Some photos do not fit the selected printer's paper and margins. Adjust their sizes or choose a larger paper."
    );
    anyhow::ensure!(!result.pages.is_empty(), "There are no photos to print");
    Ok(result)
}
impl Prepared {
    pub fn submit(self, layout: &Layout, sources: &[Source]) -> Result<()> {
        self.session.submit(layout, sources)
    }
}
#[cfg(target_os = "linux")]
pub fn helper() -> Result<()> {
    linux::helper()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borderless_requires_driver_support_and_reflows_to_final_paper_size() {
        let settings = Settings {
            default_size: PrintSize::FullPage,
            borderless: true,
            ..Settings::default()
        };
        let photos = [Photo {
            id: 1,
            source: 0,
            source_landscape: false,
            override_size: None,
            fit: Fit::Fill,
        }];
        let mut geometry = Geometry {
            paper: Size::mm(100, 150),
            hardware: Insets::default(),
            scale_percent: 100.0,
            pages_per_sheet: 1,
        };
        let layout = reconcile(&photos, &settings, &geometry).unwrap();
        let rect = layout.pages[0].photos[0].rect;
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (0, 0, 100000, 150000));
        geometry.hardware.right = 3000;
        assert!(
            reconcile(&photos, &settings, &geometry)
                .unwrap_err()
                .to_string()
                .contains("borderless")
        );
        geometry.hardware = Insets::default();
        geometry.scale_percent = 97.0;
        assert!(reconcile(&photos, &settings, &geometry).is_err());
    }
    #[test]
    fn driver_changes_repack_without_resizing_and_reject_conflicting_scale() {
        let photos = vec![
            Photo {
                id: 1,
                source: 0,
                source_landscape: false,
                override_size: None,
                fit: Fit::Fill,
            },
            Photo {
                id: 2,
                source: 0,
                source_landscape: false,
                override_size: None,
                fit: Fit::Fill,
            },
        ];
        let settings = Settings {
            default_size: PrintSize::Fixed(Size::mm(100, 150)),
            borderless: false,
            ..Settings::default()
        };
        let original = arrange(&photos, &settings, Insets::default());
        let mut geometry = Geometry {
            paper: Size::mm(148, 210),
            hardware: Insets::uniform(3000),
            scale_percent: 100.0,
            pages_per_sheet: 1,
        };
        let result = reconcile(&photos, &settings, &geometry).unwrap();
        assert_ne!(result, original);
        assert_eq!(result.pages.len(), 2);
        for p in result.pages.iter().flat_map(|p| &p.photos) {
            assert!(
                Size {
                    w: p.rect.w,
                    h: p.rect.h
                } == settings.default_size.fixed_or(settings.paper)
                    || Size {
                        w: p.rect.h,
                        h: p.rect.w
                    } == settings.default_size.fixed_or(settings.paper)
            );
        }
        geometry.scale_percent = 90.0;
        assert!(reconcile(&photos, &settings, &geometry).is_err());
        geometry.scale_percent = 100.0;
        geometry.pages_per_sheet = 2;
        assert!(reconcile(&photos, &settings, &geometry).is_err());
        geometry.pages_per_sheet = 1;
        geometry.paper = Size::mm(50, 50);
        assert!(reconcile(&photos, &settings, &geometry).is_err());
    }
}
