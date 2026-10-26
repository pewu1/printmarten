use serde::{Deserialize, Serialize};

/// All layout coordinates are integer micrometers, independent of screen/printer DPI.
pub type Length = i64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub w: Length,
    pub h: Length,
}
impl Size {
    pub const fn mm(w: i64, h: i64) -> Self {
        Self {
            w: w * 1000,
            h: h * 1000,
        }
    }
    pub fn valid(self) -> bool {
        self.w > 0 && self.h > 0 && self.w <= 2_000_000 && self.h <= 2_000_000
    }
    pub fn rotated(self) -> Self {
        Self {
            w: self.h,
            h: self.w,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    Mm,
    Cm,
    Inch,
}
impl Unit {
    pub const ALL: [Self; 3] = [Self::Mm, Self::Cm, Self::Inch];
    pub fn label(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Cm => "cm",
            Self::Inch => "in",
        }
    }
    pub fn factor(self) -> f64 {
        match self {
            Self::Mm => 1000.0,
            Self::Cm => 10_000.0,
            Self::Inch => 25_400.0,
        }
    }
    pub fn parse(self, text: &str) -> anyhow::Result<Length> {
        let value: f64 = text.trim().replace(',', ".").parse()?;
        anyhow::ensure!(
            value.is_finite() && value > 0.0,
            "Dimensions must be positive numbers"
        );
        let result = (value * self.factor()).round();
        anyhow::ensure!(
            (1.0..=2_000_000.0).contains(&result),
            "Dimensions must be between 0.001 and 2000 mm"
        );
        Ok(result as Length)
    }
    pub fn format(self, value: Length) -> String {
        format!("{:.6}", value as f64 / self.factor())
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

pub const PHOTO_PRESETS: &[(&str, Size)] = &[
    ("Passport / ID · 35 × 45 mm", Size::mm(35, 45)),
    ("Passport / ID · 2 × 2 in", Size { w: 50800, h: 50800 }),
    ("ID · 25 × 35 mm", Size::mm(25, 35)),
    ("ID · 30 × 40 mm", Size::mm(30, 40)),
    ("ID · 33 × 48 mm", Size::mm(33, 48)),
    ("Wallet · 2 × 3 in", Size { w: 50800, h: 76200 }),
    ("Wallet · 2.5 × 3.5 in", Size { w: 63500, h: 88900 }),
    ("9 × 13 cm", Size::mm(90, 130)),
    ("10 × 15 cm", Size::mm(100, 150)),
    ("13 × 18 cm", Size::mm(130, 180)),
    ("15 × 20 cm", Size::mm(150, 200)),
    ("20 × 25 cm", Size::mm(200, 250)),
    ("20 × 30 cm", Size::mm(200, 300)),
    (
        "3.5 × 5 in",
        Size {
            w: 88900,
            h: 127000,
        },
    ),
    (
        "6 × 8 in",
        Size {
            w: 152400,
            h: 203200,
        },
    ),
    (
        "8 × 12 in",
        Size {
            w: 203200,
            h: 304800,
        },
    ),
    (
        "4 × 6 in",
        Size {
            w: 101600,
            h: 152400,
        },
    ),
    (
        "5 × 7 in",
        Size {
            w: 127000,
            h: 177800,
        },
    ),
    (
        "8 × 10 in",
        Size {
            w: 203200,
            h: 254000,
        },
    ),
];
pub const PAPER_PRESETS: &[(&str, Size)] = &[
    ("A3", Size::mm(297, 420)),
    ("A4", Size::mm(210, 297)),
    ("A5", Size::mm(148, 210)),
    ("A6", Size::mm(105, 148)),
    (
        "Letter",
        Size {
            w: 215900,
            h: 279400,
        },
    ),
    (
        "Legal",
        Size {
            w: 215900,
            h: 355600,
        },
    ),
    ("10 × 15 cm", Size::mm(100, 150)),
    ("13 × 18 cm", Size::mm(130, 180)),
    (
        "4 × 6 in",
        Size {
            w: 101600,
            h: 152400,
        },
    ),
    (
        "5 × 7 in",
        Size {
            w: 127000,
            h: 177800,
        },
    ),
];
// Print-size presets also contain small ID and wallet formats, which do not
// imply that the sheet itself is photo stock.
const PHOTO_PAPER_SIZES: &[Size] = &[
    Size::mm(90, 130),
    Size::mm(100, 150),
    Size::mm(130, 180),
    Size::mm(150, 200),
    Size::mm(200, 250),
    Size::mm(200, 300),
    Size {
        w: 88900,
        h: 127000,
    },
    Size {
        w: 101600,
        h: 152400,
    },
    Size {
        w: 127000,
        h: 177800,
    },
    Size {
        w: 152400,
        h: 203200,
    },
    Size {
        w: 203200,
        h: 254000,
    },
    Size {
        w: 203200,
        h: 304800,
    },
];
pub fn size_label(size: Size) -> String {
    format!(
        "{} × {} mm",
        Unit::Mm.format(size.w),
        Unit::Mm.format(size.h)
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fit {
    #[default]
    Fill,
    Contain,
}

/// A print can use the printable sheet or an exact physical rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrintSize {
    #[default]
    FullPage,
    Fixed(Size),
}
impl PrintSize {
    pub fn label(self) -> String {
        match self {
            Self::FullPage => "Full page".into(),
            Self::Fixed(size) => size_label(size),
        }
    }
    pub fn fixed_or(self, fallback: Size) -> Size {
        match self {
            Self::FullPage => fallback,
            Self::Fixed(size) => size,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Photo {
    pub id: u64,
    pub source: usize,
    #[serde(default)]
    pub source_landscape: bool,
    pub override_size: Option<PrintSize>,
    pub fit: Fit,
}
impl Photo {
    pub fn size(&self, default: PrintSize) -> PrintSize {
        self.override_size.unwrap_or(default)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Insets {
    pub left: Length,
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
}
impl Insets {
    pub fn uniform(value: Length) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }
    pub fn max(self, other: Self) -> Self {
        Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub paper: Size,
    #[serde(default)]
    pub photo_paper: bool,
    #[serde(default)]
    pub borderless: bool,
    pub default_size: PrintSize,
    pub crop_to_fill: bool,
    pub margin: Length,
    pub gap: Length,
    pub cut_lines: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            paper: Size::mm(210, 297),
            photo_paper: true,
            borderless: true,
            default_size: PrintSize::FullPage,
            crop_to_fill: true,
            margin: 5000,
            gap: 2000,
            cut_lines: false,
        }
    }
}

impl Settings {
    pub fn default_fit(&self) -> Fit {
        if self.crop_to_fill {
            Fit::Fill
        } else {
            Fit::Contain
        }
    }

    pub fn set_photo_paper(&mut self, photo_paper: bool) {
        self.photo_paper = photo_paper;
        self.borderless = photo_paper;
    }

    pub fn select_paper(&mut self, paper: Size) {
        self.paper = paper;
        self.set_photo_paper(
            PHOTO_PAPER_SIZES
                .iter()
                .any(|size| *size == paper || size.rotated() == paper),
        );
    }

    pub fn layout_margin(&self) -> Length {
        if self.borderless {
            0
        } else {
            self.margin.max(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_defaults_are_full_page_photo_stock_borderless_and_crop() {
        let settings = Settings::default();
        assert_eq!(settings.paper, Size::mm(210, 297));
        assert_eq!(settings.default_size, PrintSize::FullPage);
        assert!(settings.photo_paper && settings.borderless && settings.crop_to_fill);
        assert_eq!(settings.default_fit(), Fit::Fill);
    }
    #[test]
    fn photo_paper_defaults_to_borderless_and_preserves_saved_margins() {
        let mut settings = Settings::default();
        settings.select_paper(Size::mm(150, 100));
        assert!(settings.photo_paper && settings.borderless);
        assert_eq!(settings.layout_margin(), 0);
        settings.borderless = false;
        assert_eq!(settings.layout_margin(), 5000);
        settings.select_paper(Size::mm(210, 297));
        assert!(!settings.photo_paper && !settings.borderless);
        settings.set_photo_paper(true);
        assert!(settings.borderless); // Photo stock can also be A4 or custom-sized.
        assert_eq!(settings.margin, 5000);
        settings.select_paper(Size::mm(35, 45));
        assert!(!settings.photo_paper && !settings.borderless);
    }
    #[test]
    fn passport_and_wallet_presets_have_exact_physical_dimensions() {
        assert_eq!(
            PHOTO_PRESETS
                .iter()
                .find(|(name, _)| *name == "Passport / ID · 35 × 45 mm")
                .unwrap()
                .1,
            Size::mm(35, 45)
        );
        assert_eq!(
            PHOTO_PRESETS
                .iter()
                .find(|(name, _)| *name == "Passport / ID · 2 × 2 in")
                .unwrap()
                .1,
            Size { w: 50800, h: 50800 }
        );
        assert_eq!(
            PHOTO_PRESETS
                .iter()
                .find(|(name, _)| *name == "Wallet · 2 × 3 in")
                .unwrap()
                .1,
            Size { w: 50800, h: 76200 }
        );
        assert!(PHOTO_PRESETS.iter().all(|(_, size)| size.valid()));
    }
    #[test]
    fn conversions_are_exact_and_accept_decimal_comma() {
        assert_eq!(Unit::Inch.parse("4").unwrap(), 101600);
        assert_eq!(Unit::Cm.parse("10,5").unwrap(), 105000);
        for unit in Unit::ALL {
            for value in [1, 12345, 127000, 2000000] {
                assert_eq!(unit.parse(&unit.format(value)).unwrap(), value);
            }
        }
        for value in ["0", "-1", "NaN", "inf", "2001"] {
            assert!(Unit::Mm.parse(value).is_err());
        }
    }
    #[test]
    fn explicit_override_survives_new_default() {
        let mut photo = Photo {
            id: 1,
            source: 0,
            source_landscape: false,
            override_size: None,
            fit: Fit::Fill,
        };
        assert_eq!(
            photo.size(PrintSize::Fixed(Size::mm(90, 130))),
            PrintSize::Fixed(Size::mm(90, 130))
        );
        photo.override_size = Some(PrintSize::Fixed(Size::mm(40, 60)));
        assert_eq!(
            photo.size(PrintSize::Fixed(Size::mm(100, 150))),
            PrintSize::Fixed(Size::mm(40, 60))
        );
        let mut copy = photo.clone();
        copy.override_size = None;
        assert_eq!(
            copy.size(PrintSize::Fixed(Size::mm(100, 150))),
            PrintSize::Fixed(Size::mm(100, 150))
        );
        assert_eq!(
            photo.override_size,
            Some(PrintSize::Fixed(Size::mm(40, 60)))
        );
    }
}
