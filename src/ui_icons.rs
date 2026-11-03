//! Bundled Lucide 0.468.0 SVGs, tinted using the current native UI palette.
use eframe::egui::{self, Color32, Vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    AddPhotos,
    AddFolder,
    Trash,
    Print,
    Copy,
    Close,
    Crop,
    FullPage,
    Ruler,
    Sheet,
    Photos,
    Zoom,
    Info,
    Link,
    Check,
    Reset,
    Portrait,
    Landscape,
    Scissors,
    Warning,
}
impl Icon {
    pub fn image(self, color: Color32) -> egui::Image<'static> {
        let source = match self {
            Self::AddPhotos => egui::include_image!("../assets/lucide/image-plus.svg"),
            Self::AddFolder => egui::include_image!("../assets/lucide/folder-plus.svg"),
            Self::Trash => egui::include_image!("../assets/lucide/trash-2.svg"),
            Self::Print => egui::include_image!("../assets/lucide/printer.svg"),
            Self::Copy => egui::include_image!("../assets/lucide/copy.svg"),
            Self::Close => egui::include_image!("../assets/lucide/x.svg"),
            Self::Crop => egui::include_image!("../assets/lucide/crop.svg"),
            Self::FullPage => egui::include_image!("../assets/lucide/scan.svg"),
            Self::Ruler => egui::include_image!("../assets/lucide/ruler.svg"),
            Self::Sheet => egui::include_image!("../assets/lucide/layers.svg"),
            Self::Photos => egui::include_image!("../assets/lucide/images.svg"),
            Self::Zoom => egui::include_image!("../assets/lucide/zoom-in.svg"),
            Self::Info => egui::include_image!("../assets/lucide/info.svg"),
            Self::Link => egui::include_image!("../assets/lucide/external-link.svg"),
            Self::Check => egui::include_image!("../assets/lucide/check.svg"),
            Self::Reset => egui::include_image!("../assets/lucide/rotate-ccw.svg"),
            Self::Portrait => egui::include_image!("../assets/lucide/rectangle-vertical.svg"),
            Self::Landscape => egui::include_image!("../assets/lucide/rectangle-horizontal.svg"),
            Self::Scissors => egui::include_image!("../assets/lucide/scissors.svg"),
            Self::Warning => egui::include_image!("../assets/lucide/circle-alert.svg"),
        };
        egui::Image::new(source)
            .fit_to_exact_size(Vec2::splat(17.0))
            .tint(color)
    }
}

pub fn button(ui: &egui::Ui, icon: Icon, label: &str) -> egui::Button<'static> {
    egui::Button::image_and_text(icon.image(ui.visuals().text_color()), label.to_owned())
        .min_size(Vec2::new(0.0, 30.0))
        .image_tint_follows_text_color(true)
        .wrap_mode(egui::TextWrapMode::Extend)
}
/// Reserve the tallest control's height before placing any labels or icons.
/// egui's default horizontal row starts at `interact_size.y` and cannot
/// recenter earlier widgets when a later widget makes the row taller.
pub fn row<R>(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let height = [
        egui::TextStyle::Body,
        egui::TextStyle::Button,
        egui::TextStyle::Monospace,
    ]
    .iter()
    .map(|style| ui.text_style_height(style))
    .fold(0.0_f32, f32::max)
        + ui.spacing().button_padding.y * 2.0;
    let height = height.max(30.0).max(ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            // Composite widgets (notably Slider) create their own horizontal rows.
            ui.spacing_mut().interact_size.y = height;
            contents(ui)
        },
    )
}

pub fn heading(ui: &mut egui::Ui, icon: Icon, label: &str) {
    let height = ui.text_style_height(&egui::TextStyle::Heading).max(17.0);
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add(icon.image(ui.visuals().text_color()));
            ui.heading(label);
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_bundled_icon_loads_through_the_svg_renderer() {
        let context = egui::Context::default();
        egui_extras::install_image_loaders(&context);
        for icon in [
            Icon::AddPhotos,
            Icon::AddFolder,
            Icon::Trash,
            Icon::Print,
            Icon::Copy,
            Icon::Close,
            Icon::Crop,
            Icon::FullPage,
            Icon::Ruler,
            Icon::Sheet,
            Icon::Photos,
            Icon::Zoom,
            Icon::Info,
            Icon::Link,
            Icon::Check,
            Icon::Reset,
            Icon::Portrait,
            Icon::Landscape,
            Icon::Scissors,
            Icon::Warning,
        ] {
            assert!(matches!(
                icon.image(Color32::BLACK)
                    .load_for_size(&context, Vec2::splat(17.0)),
                Ok(egui::load::TexturePoll::Ready { .. })
            ));
        }
    }
}
