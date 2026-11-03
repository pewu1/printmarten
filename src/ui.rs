use crate::ui_icons::{self, Icon};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use printmarten::{
    imaging::{self, Source},
    importing,
    layout::{self, Layout},
    model::*,
    printing::{self, Prepared},
};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

enum Action {
    Size(PrintSize),
    Custom,
    Reset,
    Fit(Fit),
    Copy,
    Remove,
}
enum EditTarget {
    Default,
    Paper,
    Selection(Vec<u64>),
}
struct SizeEditor {
    target: EditTarget,
    unit: Unit,
    width: String,
    height: String,
    error: String,
}
struct Textures {
    normal: egui::TextureHandle,
    rotated: egui::TextureHandle,
}
type ImportResult = (
    PathBuf,
    anyhow::Result<(Source, image::RgbImage, image::RgbImage)>,
);
enum ImportEvent {
    Discovered { files: usize, errors: Vec<String> },
    Loaded(ImportResult),
}

pub struct App {
    #[cfg(all(target_os = "macos", not(test)))]
    menu: NativeMenu,
    brand_icon: egui::TextureHandle,
    settings: Settings,
    photos: Vec<Photo>,
    sources: Vec<Source>,
    textures: Vec<Textures>,
    selection: HashSet<u64>,
    next_id: u64,
    layout: Layout,
    prepared: Option<Prepared>,
    unit: Unit,
    editor: Option<SizeEditor>,
    zoom: f32,
    import: Sender<Vec<PathBuf>>,
    received: Receiver<ImportEvent>,
    pending: usize,
    memory: usize,
    notice: String,
    errors: Vec<String>,
    dirty: bool,
    last_fixed_size: Size,
    about_open: bool,
}
#[cfg(all(target_os = "macos", not(test)))]
struct NativeMenu {
    _menu: muda::Menu,
    submenu: muda::Submenu,
    items: Vec<muda::MenuItem>,
    change_visible: bool,
}
#[cfg(all(target_os = "macos", not(test)))]
impl NativeMenu {
    fn new(ctx: egui::Context) -> Self {
        let menu = muda::Menu::new();
        let submenu = muda::Submenu::new("PrintMarten", true);
        let items: Vec<_> = [
            "Add photos…",
            "Add folder…",
            "Clear job",
            "Print…",
            "About",
            "Change printer…",
        ]
        .iter()
        .enumerate()
        .map(|(i, label)| muda::MenuItem::with_id(i.to_string(), label, true, None))
        .collect();
        for item in &items[..5] {
            submenu.append(item).unwrap();
        }
        submenu
            .append(&muda::PredefinedMenuItem::quit(None))
            .unwrap();
        menu.append(&submenu).unwrap();
        menu.init_for_nsapp();
        muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            COMMANDS.lock().unwrap().push(event.id.0);
            ctx.request_repaint();
        }));
        Self {
            _menu: menu,
            submenu,
            items,
            change_visible: false,
        }
    }
}
#[cfg(all(target_os = "macos", not(test)))]
static COMMANDS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Vec<PathBuf>) -> Self {
        Self::from_context(&cc.egui_ctx, paths)
    }
    fn from_context(context: &egui::Context, paths: Vec<PathBuf>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Noto Sans".into(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/NotoSans-Regular.ttf"))
                .into(),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, "Noto Sans".into());
        }
        context.set_fonts(fonts);
        egui_extras::install_image_loaders(context);
        let mut style = (*context.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(10.0, 6.0);
        style.visuals.selection.bg_fill = Color32::from_rgb(218, 238, 248);
        context.set_style(style);
        context.set_visuals(egui::Visuals::light());
        let icon = printmarten::branding::icon(64);
        let brand_icon = context.load_texture(
            "printmarten-icon",
            egui::ColorImage::from_rgba_unmultiplied([64, 64], icon.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        let (tx, rx) = mpsc::channel::<Vec<PathBuf>>();
        let (done, received) = mpsc::sync_channel(1);
        let ctx = context.clone();
        std::thread::spawn(move || {
            for paths in rx {
                let (paths, errors) = importing::discover(paths);
                if done
                    .send(ImportEvent::Discovered {
                        files: paths.len(),
                        errors,
                    })
                    .is_err()
                {
                    break;
                }
                ctx.request_repaint();
                for path in paths {
                    let result = imaging::load(&path).map(|source| {
                        let thumb = image::imageops::thumbnail(source.pixels.as_ref(), 640, 640);
                        let rotated = image::imageops::rotate90(&thumb);
                        (source, thumb, rotated)
                    });
                    if done.send(ImportEvent::Loaded((path, result))).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            }
        });
        let settings = Settings::default();
        let layout = layout::arrange(&[], &settings, Insets::default());
        let mut app = Self {
            #[cfg(all(target_os = "macos", not(test)))]
            menu: NativeMenu::new(context.clone()),
            brand_icon,
            settings,
            photos: vec![],
            sources: vec![],
            textures: vec![],
            selection: HashSet::new(),
            next_id: 1,
            layout,
            prepared: None,
            unit: Unit::Cm,
            editor: None,
            zoom: 1.0,
            import: tx,
            received,
            pending: 0,
            memory: 0,
            notice: String::new(),
            errors: vec![],
            dirty: false,
            last_fixed_size: Size::mm(100, 150),
            about_open: false,
        };
        app.queue(paths);
        app
    }
    fn add_photos(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Photos", importing::IMAGE_EXTENSIONS)
            .pick_files()
        {
            self.queue(paths);
        }
    }
    fn add_folder(&mut self) {
        if let Some(paths) = rfd::FileDialog::new().pick_folders() {
            self.queue(paths);
        }
    }
    fn can_print(&self) -> bool {
        self.pending == 0
            && !self.photos.is_empty()
            && self.layout.rejected.is_empty()
            && self.editor.is_none()
    }
    fn clear_job(&mut self) {
        self.photos.clear();
        self.sources.clear();
        self.textures.clear();
        self.selection.clear();
        self.memory = 0;
        self.prepared = None;
        self.editor = None;
        self.notice.clear();
        self.errors.clear();
        self.rebuild();
    }
    fn queue(&mut self, paths: Vec<PathBuf>) {
        if !paths.is_empty() && self.import.send(paths).is_ok() {
            self.pending += 1;
        }
    }
    fn brand(&self, ui: &mut egui::Ui) {
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 48.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add(
                    egui::Image::new((self.brand_icon.id(), Vec2::splat(48.0)))
                        .alt_text("PrintMarten icon"),
                );
                ui.heading(
                    egui::RichText::new("PrintMarten")
                        .size(28.0)
                        .color(Color32::from_rgb(0, 115, 170)),
                );
            },
        );
    }
    fn poll_import(&mut self, ctx: &egui::Context) {
        for _ in 0..16 {
            let Ok(event) = self.received.try_recv() else {
                break;
            };
            let (path, result) = match event {
                ImportEvent::Discovered { files, errors } => {
                    self.pending = self.pending.saturating_sub(1) + files;
                    self.errors.extend(errors);
                    continue;
                }
                ImportEvent::Loaded(result) => result,
            };
            self.pending = self.pending.saturating_sub(1);
            match result {
                Ok((source, thumb, rotated)) => {
                    let bytes = source.pixels.len();
                    if self.memory + bytes > 1024 * 1024 * 1024 {
                        self.errors.push(format!("{}: session image memory exceeds 1 GB; clear the job before importing more",path.display()));
                        continue;
                    }
                    self.memory += bytes;
                    let load = |image: &image::RgbImage, name: String| {
                        ctx.load_texture(
                            name,
                            egui::ColorImage::from_rgb(
                                [image.width() as usize, image.height() as usize],
                                image.as_raw(),
                            ),
                            egui::TextureOptions::LINEAR,
                        )
                    };
                    let source_index = self.sources.len();
                    self.textures.push(Textures {
                        normal: load(&thumb, format!("photo-{source_index}")),
                        rotated: load(&rotated, format!("photo-{source_index}-rotated")),
                    });
                    let source_landscape = source.pixels.width() > source.pixels.height();
                    self.sources.push(source);
                    self.photos.push(Photo {
                        id: self.next_id,
                        source: source_index,
                        source_landscape,
                        override_size: None,
                        fit: self.settings.default_fit(),
                    });
                    self.next_id += 1;
                    self.dirty = true;
                }
                Err(error) => self.errors.push(format!("{}: {error:#}", path.display())),
            }
        }
    }
    fn rebuild(&mut self) {
        self.prepared = None;
        self.layout = layout::arrange(&self.photos, &self.settings, Insets::default());
        self.dirty = false;
    }
    fn start_edit(&mut self, target: EditTarget, size: Size) {
        self.editor = Some(SizeEditor {
            target,
            unit: self.unit,
            width: self.unit.format(size.w),
            height: self.unit.format(size.h),
            error: String::new(),
        });
    }
    fn action(&mut self, action: Action) {
        match action {
            Action::Custom => {
                if let Some(p) = self.photos.iter().find(|p| self.selection.contains(&p.id)) {
                    self.start_edit(
                        EditTarget::Selection(self.selection.iter().copied().collect()),
                        p.size(self.settings.default_size)
                            .fixed_or(self.last_fixed_size),
                    );
                }
                return;
            }
            Action::Size(size) => {
                for p in &mut self.photos {
                    if self.selection.contains(&p.id) {
                        p.override_size = Some(size);
                    }
                }
            }
            Action::Reset => {
                for p in &mut self.photos {
                    if self.selection.contains(&p.id) {
                        p.override_size = None;
                    }
                }
            }
            Action::Fit(fit) => {
                for p in &mut self.photos {
                    if self.selection.contains(&p.id) {
                        p.fit = fit;
                    }
                }
            }
            Action::Copy => {
                let copies: Vec<_> = self
                    .photos
                    .iter()
                    .filter(|p| self.selection.contains(&p.id))
                    .cloned()
                    .collect();
                for mut photo in copies {
                    photo.id = self.next_id;
                    self.next_id += 1;
                    self.photos.push(photo);
                }
            }
            Action::Remove => {
                self.photos.retain(|p| !self.selection.contains(&p.id));
                self.selection.clear();
            }
        }
        self.dirty = true;
    }
    fn context_menu(ui: &mut egui::Ui) -> Option<Action> {
        let mut result = None;
        ui.menu_button("Print size", |ui| {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    if ui
                        .add(ui_icons::button(ui, Icon::FullPage, "Full page"))
                        .clicked()
                    {
                        result = Some(Action::Size(PrintSize::FullPage));
                        ui.close();
                    }
                    for &(name, size) in PHOTO_PRESETS {
                        if ui.button(name).clicked() {
                            result = Some(Action::Size(PrintSize::Fixed(size)));
                            ui.close();
                        }
                    }
                });
            if ui
                .add(ui_icons::button(ui, Icon::Ruler, "Custom size…"))
                .clicked()
            {
                result = Some(Action::Custom);
                ui.close();
            }
            if ui
                .add(ui_icons::button(ui, Icon::Reset, "Reset to default size"))
                .clicked()
            {
                result = Some(Action::Reset);
                ui.close();
            }
        });
        if ui
            .add(ui_icons::button(ui, Icon::Crop, "Crop to fill"))
            .clicked()
        {
            result = Some(Action::Fit(Fit::Fill));
            ui.close();
        }
        if ui
            .add(ui_icons::button(ui, Icon::FullPage, "Fit whole image"))
            .clicked()
        {
            result = Some(Action::Fit(Fit::Contain));
            ui.close();
        }
        ui.separator();
        if ui
            .add(ui_icons::button(ui, Icon::Copy, "Add one copy"))
            .clicked()
        {
            result = Some(Action::Copy);
            ui.close();
        }
        if ui
            .add(ui_icons::button(ui, Icon::Trash, "Remove"))
            .clicked()
        {
            result = Some(Action::Remove);
            ui.close();
        }
        result
    }
    fn select(&mut self, id: u64, add: bool) {
        if add {
            if !self.selection.insert(id) {
                self.selection.remove(&id);
            }
        } else {
            self.selection.clear();
            self.selection.insert(id);
        }
    }
    fn set_crop_to_fill(&mut self, enabled: bool) {
        self.settings.crop_to_fill = enabled;
        let fit = self.settings.default_fit();
        for photo in &mut self.photos {
            photo.fit = fit;
        }
        self.dirty = true;
    }
    fn has_fixed_photos(&self) -> bool {
        if self.photos.is_empty() {
            matches!(self.settings.default_size, PrintSize::Fixed(_))
        } else {
            self.photos
                .iter()
                .any(|p| matches!(p.size(self.settings.default_size), PrintSize::Fixed(_)))
        }
    }
    fn toolbar_actions(&mut self, ui: &mut egui::Ui) -> bool {
        let mut print = false;
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            if ui
                .add(ui_icons::button(ui, Icon::AddPhotos, "Add photos…"))
                .clicked()
            {
                self.add_photos();
            }
            if ui
                .add(ui_icons::button(ui, Icon::AddFolder, "Add folder…"))
                .clicked()
            {
                self.add_folder();
            }
            if ui
                .add_enabled(
                    self.pending == 0 && !self.photos.is_empty(),
                    ui_icons::button(ui, Icon::Trash, "Clear job"),
                )
                .clicked()
            {
                self.clear_job();
            }
            let label = if self.prepared.is_some() {
                "Print reviewed layout"
            } else {
                "Print…"
            };
            let enabled = self.can_print();
            if ui
                .add_enabled(
                    enabled,
                    ui_icons::button(ui, Icon::Print, label).fill(Color32::from_rgb(210, 235, 247)),
                )
                .clicked()
            {
                print = true;
            }
            if self.prepared.is_some()
                && ui
                    .add(ui_icons::button(ui, Icon::Print, "Change printer…"))
                    .clicked()
            {
                self.prepared = None;
                print = true;
            }
        });
        print
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) -> bool {
        // Measure text in the active font so the action group can be centered
        // independently of the branding and right-hand counts.
        let mut labels = vec![
            "Add photos…",
            "Add folder…",
            "Clear job",
            if self.prepared.is_some() {
                "Print reviewed layout"
            } else {
                "Print…"
            },
        ];
        if self.prepared.is_some() {
            labels.push("Change printer…");
        }
        let font = egui::TextStyle::Button.resolve(ui.style());
        let action_width: f32 = labels
            .iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap((*label).into(), font.clone(), Color32::BLACK)
                    .size()
                    .x
                    + ui.spacing().button_padding.x * 2.0
                    + 17.0_f32.min(ui.text_style_height(&egui::TextStyle::Button))
                    + ui.spacing().icon_spacing
            })
            .sum::<f32>()
            + ui.spacing().item_spacing.x * (labels.len() - 1) as f32;
        let side = 245.0_f32.max(if self.pending > 0 { 290.0 } else { 245.0 });
        let two_rows = ui.available_width() < action_width + side * 2.0 + 24.0;
        let (header, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), egui::Sense::hover());
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(header.min, Vec2::new(side, 56.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| self.brand(ui),
        );
        let right = Rect::from_min_max(Pos2::new(header.right() - side, header.top()), header.max);
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(right)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                if ui
                    .add(ui_icons::button(ui, Icon::Info, "About"))
                    .on_hover_text("About PrintMarten")
                    .clicked()
                {
                    self.about_open = true;
                }
                let row_height = ui.text_style_height(&egui::TextStyle::Body).max(17.0);
                let height = if self.pending > 0 {
                    row_height * 2.0 + ui.spacing().item_spacing.y
                } else {
                    row_height
                };
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().interact_size.y = row_height;
                        ui.horizontal(|ui| {
                            ui.add(Icon::Photos.image(ui.visuals().text_color()));
                            ui.label(format!(
                                "{} photos · {} sheets",
                                self.photos.len(),
                                self.layout.pages.len()
                            ));
                        });
                        if self.pending > 0 {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.small(format!("Importing {}…", self.pending));
                            });
                        }
                    },
                );
            },
        );
        let action_area = if two_rows {
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), egui::Sense::hover())
                .0
        } else {
            header
        };
        let centered = Rect::from_center_size(
            action_area.center(),
            Vec2::new(action_width, action_area.height()),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(centered)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| self.toolbar_actions(ui),
        )
        .inner
    }
    fn about(&mut self, ctx: &egui::Context) {
        let mut open = self.about_open;
        egui::Window::new("About PrintMarten")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.allocate_ui(Vec2::new(235.0, 48.0), |ui| self.brand(ui));
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.label("Author: Piotr Wenta");
                    ui.allocate_ui(Vec2::new(190.0, 24.0), |ui| {
                        ui_icons::row(ui, |ui| {
                            ui.add(Icon::Link.image(ui.visuals().text_color()));
                            ui.hyperlink_to("PrintMarten on GitHub", env!("CARGO_PKG_REPOSITORY"));
                        });
                    });
                    ui.separator();
                    ui.label("Licensed under GNU GPL v3 (GPL-3.0-only)");
                    egui::ScrollArea::vertical()
                        .id_salt("about-license")
                        .max_height(240.0)
                        .show(ui, |ui| {
                            ui.add(egui::Label::new(include_str!("../LICENSE")).wrap());
                        });
                    if ui.add(ui_icons::button(ui, Icon::Close, "Close")).clicked() {
                        self.about_open = false;
                    }
                });
            });
        self.about_open &= open;
    }
    fn controls(&mut self, ui: &mut egui::Ui) {
        ui_icons::heading(ui, Icon::Sheet, "Sheet");
        let current = PAPER_PRESETS
            .iter()
            .find(|(_, s)| *s == self.settings.paper || s.rotated() == self.settings.paper)
            .map(|(n, _)| *n)
            .unwrap_or("Custom");
        egui::ComboBox::from_id_salt("paper")
            .selected_text(current)
            .width(200.0)
            .show_ui(ui, |ui| {
                for &(name, size) in PAPER_PRESETS {
                    if ui.selectable_label(current == name, name).clicked() {
                        let landscape = self.settings.paper.w > self.settings.paper.h;
                        self.settings
                            .select_paper(if landscape { size.rotated() } else { size });
                        self.dirty = true;
                    }
                }
                if ui
                    .add(ui_icons::button(ui, Icon::Ruler, "Custom paper…"))
                    .clicked()
                {
                    self.start_edit(EditTarget::Paper, self.settings.paper);
                }
            });
        ui.small(size_label(self.settings.paper));
        ui_icons::row(ui, |ui| {
            let landscape = self.settings.paper.w > self.settings.paper.h;
            if ui
                .add(ui_icons::button(ui, Icon::Portrait, "Portrait").selected(!landscape))
                .clicked()
                && landscape
            {
                self.settings.paper = self.settings.paper.rotated();
                self.dirty = true;
            }
            if ui
                .add(ui_icons::button(ui, Icon::Landscape, "Landscape").selected(landscape))
                .clicked()
                && !landscape
            {
                self.settings.paper = self.settings.paper.rotated();
                self.dirty = true;
            }
        });
        let mut photo_paper = self.settings.photo_paper;
        if ui.checkbox(&mut photo_paper, "Photo paper").changed() {
            self.settings.set_photo_paper(photo_paper);
            self.dirty = true;
        }
        if ui
            .checkbox(&mut self.settings.borderless, "Borderless")
            .changed()
        {
            self.dirty = true;
        }
        if self.settings.borderless {
            ui.small("Choose a borderless paper size or mode in the print dialog if needed.");
        }
        ui.add_space(12.0);
        ui_icons::heading(ui, Icon::Photos, "Default photo size");
        let current = match self.settings.default_size {
            PrintSize::FullPage => "Full page",
            PrintSize::Fixed(size) => PHOTO_PRESETS
                .iter()
                .find(|(_, preset)| *preset == size)
                .map(|(name, _)| *name)
                .unwrap_or("Custom"),
        };
        egui::ComboBox::from_id_salt("size")
            .selected_text(current)
            .width(220.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(
                        self.settings.default_size == PrintSize::FullPage,
                        "Full page",
                    )
                    .clicked()
                {
                    self.settings.default_size = PrintSize::FullPage;
                    self.dirty = true;
                }
                for &(name, size) in PHOTO_PRESETS {
                    if ui.selectable_label(current == name, name).clicked() {
                        self.settings.default_size = PrintSize::Fixed(size);
                        self.last_fixed_size = size;
                        self.dirty = true;
                    }
                }
                if ui
                    .add(ui_icons::button(ui, Icon::Ruler, "Custom size…"))
                    .clicked()
                {
                    self.start_edit(
                        EditTarget::Default,
                        self.settings.default_size.fixed_or(self.last_fixed_size),
                    );
                }
            });
        ui.small(self.settings.default_size.label());
        ui.small("Customized photos keep their own size.");
        if self.settings.default_size == PrintSize::FullPage {
            ui.small("One photo per sheet. Smaller overrides are packed together.");
        }
        let mut crop = self.settings.crop_to_fill;
        if ui.checkbox(&mut crop, "Crop to fill").on_hover_text("Apply to all photos and future imports. Right-click a photo to change it individually.").changed() {
            self.set_crop_to_fill(crop);
        }
        let packed = self.has_fixed_photos();
        ui.add_space(12.0);
        ui_icons::heading(ui, Icon::Scissors, "Spacing");
        let mut margin = self.settings.margin as f64 / 1000.0;
        let mut gap = self.settings.gap as f64 / 1000.0;
        ui_icons::row(ui, |ui| {
            ui.label("Min. margin");
            if ui
                .add_enabled(
                    !self.settings.borderless,
                    egui::DragValue::new(&mut margin)
                        .range(0.0..=100.0)
                        .speed(0.1)
                        .suffix(" mm"),
                )
                .changed()
            {
                self.settings.margin = (margin * 1000.0).round() as i64;
                self.dirty = true;
            }
        });
        ui_icons::row(ui, |ui| {
            ui.label("Min. gap");
            if ui
                .add_enabled(
                    packed,
                    egui::DragValue::new(&mut gap)
                        .range(0.0..=100.0)
                        .speed(0.1)
                        .suffix(" mm"),
                )
                .changed()
            {
                self.settings.gap = (gap * 1000.0).round() as i64;
                self.dirty = true;
            }
        });
        ui.small(if self.settings.borderless {
            "Photos prefer sheet edges; fewer sheets takes priority."
        } else {
            "Photos are packed at their exact size with minimum spacing."
        });
        if ui
            .add_enabled(
                packed,
                egui::Checkbox::new(&mut self.settings.cut_lines, "Print cut lines"),
            )
            .changed()
        {
            self.dirty = true;
        }
    }
    fn photo_controls(&mut self, ui: &mut egui::Ui) {
        ui_icons::heading(ui, Icon::Photos, &format!("Photos ({})", self.photos.len()));
        ui.small("Ctrl/Cmd-click to select several.\nRight-click a photo for sizes and copies.");
        if !self.selection.is_empty() {
            ui_icons::row(ui, |ui| {
                if ui.add(ui_icons::button(ui, Icon::Copy, "Copy")).clicked() {
                    self.action(Action::Copy);
                }
                if ui
                    .add(ui_icons::button(ui, Icon::Trash, "Remove"))
                    .clicked()
                {
                    self.action(Action::Remove);
                }
            });
        }
    }
    fn thumbnails(&mut self, ui: &mut egui::Ui) {
        let mut action = None;
        let mut select = None;
        egui::ScrollArea::vertical()
            .id_salt("thumbnails")
            .show(ui, |ui| {
                for photo in &self.photos {
                    let source = &self.sources[photo.source];
                    let selected = self.selection.contains(&photo.id);
                    let stroke = if selected {
                        Stroke::new(2.0_f32, Color32::from_rgb(0, 130, 180))
                    } else {
                        Stroke::new(2.0_f32, Color32::TRANSPARENT)
                    };
                    let response = egui::Frame::new()
                        .inner_margin(8.0)
                        .corner_radius(6.0)
                        .stroke(stroke)
                        .fill(if selected {
                            Color32::from_rgb(232, 245, 251)
                        } else {
                            Color32::TRANSPARENT
                        })
                        .show(ui, |ui| {
                            let width = ui.available_width();
                            ui.set_min_width(width);
                            ui.allocate_ui_with_layout(
                                Vec2::new(ui.available_width(), 54.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::Image::new((
                                            self.textures[photo.source].normal.id(),
                                            Vec2::splat(54.0),
                                        ))
                                        .fit_to_exact_size(Vec2::splat(54.0)),
                                    );
                                    ui.vertical(|ui| {
                                        let label = source
                                            .path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy();
                                        ui.add(egui::Label::new(label).truncate());
                                        ui.small(format!(
                                            "{}{}",
                                            photo.size(self.settings.default_size).label(),
                                            if photo.override_size.is_some() {
                                                " •"
                                            } else {
                                                ""
                                            }
                                        ));
                                    });
                                },
                            );
                        })
                        .response
                        .interact(egui::Sense::click());
                    if response.clicked() {
                        select = Some((
                            photo.id,
                            ui.input(|i| i.modifiers.command || i.modifiers.ctrl),
                        ));
                    }
                    if response.secondary_clicked() && !selected {
                        self.selection.clear();
                        self.selection.insert(photo.id);
                    }
                    response.context_menu(|ui| {
                        action = Self::context_menu(ui);
                    });
                    ui.separator();
                }
            });
        if let Some((id, add)) = select {
            self.select(id, add);
        }
        if let Some(action) = action {
            self.action(action);
        }
    }
    fn preview(&mut self, ui: &mut egui::Ui) {
        if self.photos.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(
                        "Drop photos or folders here\nor choose Add photos to get started",
                    )
                    .size(25.0)
                    .color(Color32::from_gray(100)),
                );
            });
            return;
        }
        let mut action = None;
        let mut select = None;
        let viewport = ui.available_size();
        let fit_scale = ((viewport.x - 20.0) / self.layout.paper.w as f32)
            .min((viewport.y - 50.0) / self.layout.paper.h as f32)
            .max(0.00001);
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = self.layout.paper.w as f32 * fit_scale * self.zoom;
                let scale = width / self.layout.paper.w as f32;
                let height = self.layout.paper.h as f32 * scale;
                for (index, page) in self.layout.pages.iter().enumerate() {
                    ui.label(format!(
                        "Sheet {} of {}",
                        index + 1,
                        self.layout.pages.len()
                    ));
                    let (space, _) = ui.allocate_exact_size(
                        Vec2::new(width.max(ui.available_width()), height),
                        egui::Sense::hover(),
                    );
                    let sheet = Rect::from_center_size(space.center(), Vec2::new(width, height));
                    ui.painter().rect_filled(sheet, 0.0, Color32::WHITE);
                    ui.painter().rect_stroke(
                        sheet,
                        0.0,
                        Stroke::new(1.0_f32, Color32::from_gray(160)),
                        StrokeKind::Inside,
                    );
                    let hardware = self
                        .prepared
                        .as_ref()
                        .map(|p| p.geometry.hardware)
                        .unwrap_or_default()
                        .max(Insets::uniform(self.settings.layout_margin()));
                    let printable = Rect::from_min_max(
                        sheet.min
                            + Vec2::new(hardware.left as f32 * scale, hardware.top as f32 * scale),
                        sheet.max
                            - Vec2::new(
                                hardware.right as f32 * scale,
                                hardware.bottom as f32 * scale,
                            ),
                    );
                    if printable.is_positive() {
                        ui.painter().rect_stroke(
                            printable,
                            0.0,
                            Stroke::new(0.5_f32, Color32::from_gray(220)),
                            StrokeKind::Inside,
                        );
                    }
                    for p in &page.photos {
                        let target = Rect::from_min_size(
                            sheet.min + Vec2::new(p.rect.x as f32 * scale, p.rect.y as f32 * scale),
                            Vec2::new(p.rect.w as f32 * scale, p.rect.h as f32 * scale),
                        );
                        let source = &self.sources[p.source];
                        let dims = source.pixels.dimensions();
                        let dims = if p.rotated { (dims.1, dims.0) } else { dims };
                        let (uv, dest) =
                            layout::image_geometry(dims, (p.rect.w as f64, p.rect.h as f64), p.fit);
                        let rect = Rect::from_min_max(
                            target.min + target.size() * Vec2::new(dest[0], dest[1]),
                            target.min + target.size() * Vec2::new(dest[2], dest[3]),
                        );
                        let texture = if p.rotated {
                            &self.textures[p.source].rotated
                        } else {
                            &self.textures[p.source].normal
                        };
                        ui.painter().image(
                            texture.id(),
                            rect,
                            Rect::from_min_max(Pos2::new(uv[0], uv[1]), Pos2::new(uv[2], uv[3])),
                            Color32::WHITE,
                        );
                        let selected = self.selection.contains(&p.photo_id);
                        if selected {
                            ui.painter().rect_stroke(
                                target,
                                0.0,
                                Stroke::new(3.0_f32, Color32::from_rgb(0, 140, 190)),
                                StrokeKind::Inside,
                            );
                        }
                        let response = ui.interact(
                            target,
                            egui::Id::new(("placed", p.photo_id)),
                            egui::Sense::click(),
                        );
                        if response.clicked() {
                            select = Some((
                                p.photo_id,
                                ui.input(|i| i.modifiers.command || i.modifiers.ctrl),
                            ));
                        }
                        if response.secondary_clicked() && !selected {
                            self.selection.clear();
                            self.selection.insert(p.photo_id);
                        }
                        let response = response.on_hover_text(format!(
                            "{}\n{}\n{:.0} DPI • right-click for options",
                            source
                                .path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy(),
                            size_label(Size {
                                w: p.rect.w,
                                h: p.rect.h
                            }),
                            imaging::effective_dpi(source.pixels.dimensions(), p)
                        ));
                        response.context_menu(|ui| {
                            action = Self::context_menu(ui);
                        });
                    }
                    for line in &page.cut_lines {
                        ui.painter().line_segment(
                            [
                                sheet.min
                                    + Vec2::new(
                                        line.start[0] as f32 * scale,
                                        line.start[1] as f32 * scale,
                                    ),
                                sheet.min
                                    + Vec2::new(
                                        line.end[0] as f32 * scale,
                                        line.end[1] as f32 * scale,
                                    ),
                            ],
                            Stroke::new((0.25 * 25400.0 / 72.0 * scale).max(0.5), Color32::BLACK),
                        );
                    }
                    ui.add_space(20.0);
                }
            });
        if let Some((id, add)) = select {
            self.select(id, add);
        }
        if let Some(action) = action {
            self.action(action);
        }
    }
    fn size_editor(&mut self, ctx: &egui::Context) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        let mut apply = None;
        let mut cancel = false;
        egui::Window::new("Custom dimensions")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                let previous = editor.unit;
                egui::ComboBox::from_id_salt("editor-unit")
                    .selected_text(editor.unit.label())
                    .show_ui(ui, |ui| {
                        for unit in Unit::ALL {
                            ui.selectable_value(&mut editor.unit, unit, unit.label());
                        }
                    });
                if previous != editor.unit {
                    if let (Ok(w), Ok(h)) = (
                        previous.parse(&editor.width),
                        previous.parse(&editor.height),
                    ) {
                        editor.width = editor.unit.format(w);
                        editor.height = editor.unit.format(h);
                    } else {
                        editor.unit = previous;
                        editor.error = "Enter valid dimensions before changing units".into();
                    }
                }
                ui_icons::row(ui, |ui| {
                    ui.label("Width");
                    ui.text_edit_singleline(&mut editor.width);
                });
                ui_icons::row(ui, |ui| {
                    ui.label("Height");
                    ui.text_edit_singleline(&mut editor.height);
                });
                if !editor.error.is_empty() {
                    ui.colored_label(Color32::DARK_RED, &editor.error);
                }
                ui_icons::row(ui, |ui| {
                    if ui.add(ui_icons::button(ui, Icon::Check, "Apply")).clicked() {
                        match editor
                            .unit
                            .parse(&editor.width)
                            .and_then(|w| editor.unit.parse(&editor.height).map(|h| Size { w, h }))
                        {
                            Ok(size) => apply = Some(size),
                            Err(error) => editor.error = error.to_string(),
                        }
                    }
                    if ui
                        .add(ui_icons::button(ui, Icon::Close, "Cancel"))
                        .clicked()
                    {
                        cancel = true;
                    }
                });
            });
        if let Some(size) = apply {
            let editor = self.editor.take().unwrap();
            self.unit = editor.unit;
            match editor.target {
                EditTarget::Default => {
                    self.settings.default_size = PrintSize::Fixed(size);
                    self.last_fixed_size = size;
                }
                EditTarget::Paper => self.settings.select_paper(size),
                EditTarget::Selection(ids) => {
                    for p in &mut self.photos {
                        if ids.contains(&p.id) {
                            p.override_size = Some(PrintSize::Fixed(size));
                        }
                    }
                }
            }
            self.dirty = true;
        } else if cancel {
            self.editor = None;
        }
    }
    fn print(&mut self) {
        self.notice.clear();
        let result = (|| -> anyhow::Result<()> {
            if let Some(prepared) = self.prepared.take() {
                prepared.submit(&self.layout, &self.sources)?;
                self.notice = "Print job submitted.".into();
                return Ok(());
            }
            let Some(prepared) = printing::prepare(&self.settings, self.layout.pages.len())? else {
                self.notice = "Printing cancelled.".into();
                return Ok(());
            };
            let final_layout =
                printing::reconcile(&self.photos, &self.settings, &prepared.geometry)?;
            if final_layout != self.layout {
                self.settings.paper = prepared.geometry.paper;
                self.layout = final_layout;
                self.prepared = Some(prepared);
                self.notice="The printer settings changed the layout. Review the sheets, then choose Print reviewed layout.".into();
            } else {
                prepared.submit(&self.layout, &self.sources)?;
                self.notice = "Print job submitted.".into();
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.errors.push(format!("Printing: {error:#}"));
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_import(ctx);
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect::<Vec<_>>()
        });
        if !dropped.is_empty() {
            self.queue(dropped);
        }
        if self.dirty {
            self.rebuild();
        }
        #[cfg(all(target_os = "macos", not(test)))]
        {
            self.menu.items[2].set_enabled(self.pending == 0 && !self.photos.is_empty());
            self.menu.items[3].set_enabled(self.can_print());
            self.menu.items[3].set_text(if self.prepared.is_some() {
                "Print reviewed layout"
            } else {
                "Print…"
            });
            let visible = self.prepared.is_some();
            if visible != self.menu.change_visible {
                if visible {
                    self.menu.submenu.insert(&self.menu.items[5], 4).unwrap();
                } else {
                    self.menu.submenu.remove(&self.menu.items[5]).unwrap();
                }
                self.menu.change_visible = visible;
            }
            let commands = std::mem::take(&mut *COMMANDS.lock().unwrap());
            for id in commands {
                match id.as_str() {
                    "0" => self.add_photos(),
                    "1" => self.add_folder(),
                    "2" if self.pending == 0 => self.clear_job(),
                    "3" if self.can_print() => self.print(),
                    "4" => self.about_open = true,
                    "5" if self.can_print() => {
                        self.prepared = None;
                        self.print();
                    }
                    _ => {}
                }
            }
        }
        let mut print = false;
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            print = self.toolbar(ui);
        });
        egui::TopBottomPanel::bottom("status").show(ctx,|ui| {
            ui_icons::row(ui, |ui|{ui.add(Icon::Zoom.image(ui.visuals().text_color()));ui.label("Preview zoom");if ui.button("Fit sheet").clicked(){self.zoom=1.0;}ui.add(egui::Slider::new(&mut self.zoom,0.3..=2.0).custom_formatter(|v,_| format!("{:.0}%",v*100.0)));ui.small("Photo sizes stay exact regardless of preview zoom.");});
            if !self.notice.is_empty(){ui.label(&self.notice);}
            if !self.layout.rejected.is_empty() {
                ui.colored_label(Color32::DARK_RED,format!("{} photo(s) do not fit. Choose a larger sheet or a smaller photo size. Nothing will be shrunk.",self.layout.rejected.len()));
                for id in self.layout.rejected.iter().take(3) {
                    if let Some(photo)=self.photos.iter().find(|photo|photo.id==*id) {
                        ui.small(format!("{} — {}",self.sources[photo.source].path.file_name().unwrap_or_default().to_string_lossy(),photo.size(self.settings.default_size).label()));
                    }
                }
                if self.layout.rejected.len()>3 {
                    ui.small(format!("…and {} more. All photos remain in the thumbnail list.",self.layout.rejected.len()-3));
                }
            }
            let low=self.layout.pages.iter().flat_map(|p|&p.photos).filter(|p|imaging::effective_dpi(self.sources[p.source].pixels.dimensions(),p)<150.0).count();
            if low>0{ui.colored_label(Color32::from_rgb(150,90,0),format!("{low} photo(s) are below 150 DPI at the requested size and may look soft."));}
            if !self.errors.is_empty(){ui_icons::row(ui, |ui|{ui.add(Icon::Warning.image(Color32::DARK_RED));ui.colored_label(Color32::DARK_RED,format!("{} problem(s)",self.errors.len()));if ui.add(ui_icons::button(ui, Icon::Close, "Dismiss")).clicked(){self.errors.clear();}});egui::ScrollArea::vertical().id_salt("errors").max_height(90.0).show(ui,|ui|for error in &self.errors {ui.colored_label(Color32::DARK_RED,error);});}
        });
        egui::SidePanel::left("controls")
            .exact_width(265.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("sheet-controls")
                    .show(ui, |ui| {
                        self.controls(ui);
                    });
            });
        if self.dirty {
            self.rebuild();
        }
        egui::SidePanel::right("photos")
            .default_width(240.0)
            .width_range(180.0..=400.0)
            .resizable(true)
            .show(ctx, |ui| {
                self.photo_controls(ui);
                self.thumbnails(ui);
            });
        if self.dirty {
            self.rebuild();
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(231, 235, 240))
                    .inner_margin(18.0),
            )
            .show(ctx, |ui| self.preview(ui));
        self.size_editor(ctx);
        if self.about_open {
            self.about(ctx);
        }
        if self.dirty {
            self.rebuild();
        }
        if print {
            self.print();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
            && !ctx.wants_keyboard_input()
            && self.editor.is_none()
        {
            self.action(Action::Remove);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A))
            && !ctx.wants_keyboard_input()
        {
            self.selection = self.photos.iter().map(|p| p.id).collect();
        }
        if self.pending > 0 {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> (egui::Context, App) {
        let context = egui::Context::default();
        let mut app = App::from_context(&context, vec![]);
        let pixels = image::RgbImage::from_pixel(300, 200, image::Rgb([200, 100, 30]));
        let texture = context.load_texture(
            "test",
            egui::ColorImage::from_rgb([300, 200], pixels.as_raw()),
            Default::default(),
        );
        app.sources.push(Source {
            path: "test-photo.png".into(),
            pixels: std::sync::Arc::new(pixels),
        });
        app.textures.push(Textures {
            normal: texture.clone(),
            rotated: texture,
        });
        app.photos = (1..=3)
            .map(|id| Photo {
                id,
                source: 0,
                source_landscape: true,
                override_size: None,
                fit: Fit::Fill,
            })
            .collect();
        app.next_id = 4;
        app.rebuild();
        (context, app)
    }
    #[test]
    fn clearing_job_keeps_settings_and_renders_the_next_frame() {
        let (ctx, mut app) = app();
        let settings = app.settings.clone();
        app.clear_job();
        assert!(app.photos.is_empty() && app.sources.is_empty() && app.layout.pages.is_empty());
        assert_eq!(app.settings, settings);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.preview(ui));
        });
        app.queue(Vec::new());
        assert_eq!(app.pending, 0);
    }
    #[test]
    fn global_crop_updates_current_photos_but_individual_changes_and_copies_are_independent() {
        let (_, mut app) = app();
        app.set_crop_to_fill(false);
        assert!(app.photos.iter().all(|photo| photo.fit == Fit::Contain));
        assert_eq!(app.settings.default_fit(), Fit::Contain);
        app.select(1, false);
        app.action(Action::Fit(Fit::Fill));
        app.action(Action::Copy);
        assert_eq!(app.photos.last().unwrap().fit, Fit::Fill);
        assert_eq!(app.photos[1].fit, Fit::Contain);
        app.set_crop_to_fill(true);
        assert!(app.photos.iter().all(|photo| photo.fit == Fit::Fill));
        app.rebuild();
        assert_eq!(app.layout.pages.len(), 4);
    }

    #[test]
    fn full_page_overrides_and_reset_follow_the_new_default_and_custom_editor_keeps_units() {
        let (_, mut app) = app();
        app.settings.default_size = PrintSize::Fixed(Size::mm(40, 60));
        app.select(1, false);
        app.action(Action::Size(PrintSize::FullPage));
        app.action(Action::Copy);
        assert_eq!(
            app.photos.last().unwrap().override_size,
            Some(PrintSize::FullPage)
        );
        app.action(Action::Custom);
        let editor = app.editor.as_ref().unwrap();
        assert_eq!(editor.unit, Unit::Cm);
        assert_eq!((&*editor.width, &*editor.height), ("10", "15"));
        app.editor = None;
        app.action(Action::Reset);
        assert_eq!(
            app.photos[0].size(app.settings.default_size),
            PrintSize::Fixed(Size::mm(40, 60))
        );
        app.settings.default_size = PrintSize::FullPage;
        assert!(
            app.photos
                .iter()
                .all(|p| p.size(app.settings.default_size) == PrintSize::FullPage)
        );
    }

    #[test]
    fn importing_a_folder_loads_nested_photos_and_reports_corrupt_images() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("nested")).unwrap();
        image::RgbImage::new(40, 20)
            .save(directory.path().join("a.png"))
            .unwrap();
        image::RgbImage::new(20, 40)
            .save(directory.path().join("nested/b.png"))
            .unwrap();
        std::fs::write(directory.path().join("broken.jpg"), b"invalid").unwrap();
        std::fs::write(directory.path().join("notes.txt"), b"ignored").unwrap();
        let context = egui::Context::default();
        let mut app = App::from_context(&context, vec![directory.path().into()]);
        app.set_crop_to_fill(false);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.pending > 0 && std::time::Instant::now() < deadline {
            app.poll_import(&context);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(app.pending, 0);
        assert_eq!(app.photos.len(), 2);
        assert!(app.photos.iter().all(|photo| photo.fit == Fit::Contain));
        assert_eq!(app.errors.len(), 1);
        assert!(app.errors[0].contains("broken.jpg"));
        assert!(app.photos[0].source_landscape);
        assert!(!app.photos[1].source_landscape);
        app.rebuild();
        assert!(app.layout.pages[0].photos[0].rotated);
    }
    #[test]
    fn multi_selection_copy_and_reset_leave_other_instances_unchanged() {
        let (_, mut app) = app();
        app.select(1, false);
        app.select(2, true);
        app.action(Action::Size(PrintSize::Fixed(Size::mm(60, 90))));
        app.action(Action::Copy);
        assert_eq!(app.photos.len(), 5);
        assert_eq!(app.photos[2].override_size, None);
        app.select(4, false);
        app.action(Action::Size(PrintSize::Fixed(Size::mm(40, 50))));
        app.settings.default_size = PrintSize::Fixed(Size::mm(130, 180));
        app.rebuild();
        assert_eq!(
            app.photos[0].size(app.settings.default_size),
            PrintSize::Fixed(Size::mm(60, 90))
        );
        assert_eq!(
            app.photos[2].size(app.settings.default_size),
            PrintSize::Fixed(Size::mm(130, 180))
        );
        assert_eq!(
            app.photos[3].size(app.settings.default_size),
            PrintSize::Fixed(Size::mm(40, 50))
        );
        app.action(Action::Reset);
        assert_eq!(app.photos[3].override_size, None);
        app.action(Action::Remove);
        assert_eq!(app.photos.len(), 4);
        assert!(app.selection.is_empty());
    }
    fn text_rect(shapes: &[egui::epaint::ClippedShape], label: &str) -> Rect {
        fn find(shape: &egui::Shape, label: &str) -> Option<Rect> {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(Rect::from_min_size(text.pos, text.galley.size()))
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
                _ => None,
            }
        }
        shapes
            .iter()
            .find_map(|shape| find(&shape.shape, label))
            .unwrap_or_else(|| panic!("missing text: {label}"))
    }

    #[test]
    fn header_actions_are_centered_counts_are_right_and_about_uses_the_correct_brand() {
        for width in [800.0, 1180.0] {
            let (context, mut app) = app();
            let mut output = None;
            for _ in 0..2 {
                output = Some(context.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 820.0))),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::TopBottomPanel::top("test-toolbar").show(ctx, |ui| {
                            app.toolbar(ui);
                        });
                    },
                ));
            }
            let shapes = &output.unwrap().shapes;
            let brand = text_rect(shapes, "PrintMarten");
            let add = text_rect(shapes, "Add photos…");
            let clear = text_rect(shapes, "Clear job");
            let print = text_rect(shapes, "Print…");
            let counts = text_rect(shapes, "3 photos · 3 sheets");
            assert!(brand.height() >= 27.0);
            let about = text_rect(shapes, "About");
            assert!((brand.center().y - counts.center().y).abs() <= 1.0);
            assert!((brand.center().y - about.center().y).abs() <= 1.0);
            if width == 1180.0 {
                assert!((brand.center().y - add.center().y).abs() <= 1.0);
            }
            assert!(counts.left() > width * 0.7);
            assert!(clear.left() > add.right() && print.left() > clear.right());
            // Include the first button's icon and symmetric button padding.
            let center = (add.left() - 14.0 - 4.0 + print.right()) / 2.0;
            assert!(
                (center - width / 2.0).abs() < 6.0,
                "action center {center} at width {width}"
            );
            if width == 800.0 {
                assert!(add.top() > brand.bottom());
            }
            app.about_open = true;
            let mut output = None;
            for _ in 0..2 {
                output = Some(context.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 820.0))),
                        ..Default::default()
                    },
                    |ctx| app.about(ctx),
                ));
            }
            let shapes = &output.unwrap().shapes;
            text_rect(shapes, "Author: Piotr Wenta");
            text_rect(shapes, "PrintMarten on GitHub");
            text_rect(shapes, "Licensed under GNU GPL v3 (GPL-3.0-only)");
        }
    }

    #[test]
    fn mixed_controls_share_a_vertical_center() {
        let (context, _) = app();
        let mut dimensions = "10".to_owned();
        let mut margin = 2.0;
        let mut zoom = 1.0;
        let _ = context.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for field in 0..3 {
                    ui_icons::row(ui, |ui| {
                        let icon = ui.add(Icon::Ruler.image(Color32::BLACK)).rect;
                        let label = ui.label("Value").rect;
                        let input = match field {
                            0 => ui.text_edit_singleline(&mut dimensions).rect,
                            1 => ui.add(egui::DragValue::new(&mut margin)).rect,
                            _ => ui.add(egui::Slider::new(&mut zoom, 0.3..=2.0)).rect,
                        };
                        let button = ui.add(ui_icons::button(ui, Icon::Check, "Apply")).rect;
                        for rect in [label, input, button] {
                            assert!(
                                (rect.center().y - icon.center().y).abs() <= 1.0,
                                "control {field}: icon {icon:?}, widget {rect:?}",
                            );
                        }
                    });
                }
            });
        });
    }

    #[test]
    fn headless_preview_and_context_controls_render_after_mixed_edits() {
        let (context, mut app) = app();
        app.select(2, false);
        app.action(Action::Fit(Fit::Contain));
        app.action(Action::Size(PrintSize::Fixed(Size::mm(60, 60))));
        app.rebuild();
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 800.0))),
                ..Default::default()
            },
            |ctx| {
                egui::TopBottomPanel::top("test-brand").show(ctx, |ui| app.brand(ui));
                egui::SidePanel::left("test-controls")
                    .exact_width(265.0)
                    .show(ctx, |ui| {
                        app.controls(ui);
                        app.thumbnails(ui);
                    });
                egui::CentralPanel::default().show(ctx, |ui| app.preview(ui));
            },
        );
        assert!(!output.shapes.is_empty());
        assert!(app.layout.rejected.is_empty());
    }
}
