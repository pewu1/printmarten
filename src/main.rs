#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod ui;

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.iter().any(|s| s == "--version") {
        println!(
            "PrintMarten {} ({}/{})",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        return Ok(());
    }
    if args.first().is_some_and(|s| s == "--check-image") {
        anyhow::ensure!(args.len() == 2, "Usage: printmarten --check-image FILE");
        let source = printmarten::imaging::load(std::path::Path::new(&args[1]))?;
        println!(
            "{} × {} pixels",
            source.pixels.width(),
            source.pixels.height()
        );
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    if args.iter().any(|s| s == "--print-helper") {
        return printmarten::printing::helper();
    }
    let icon = image::load_from_memory(include_bytes!("../icon.jpg"))?
        .thumbnail(256, 256)
        .to_rgba8();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("PrintMarten")
            .with_inner_size([1180.0, 820.0])
            .with_min_inner_size([800.0, 560.0])
            .with_icon(eframe::egui::IconData {
                width: icon.width(),
                height: icon.height(),
                rgba: icon.into_raw(),
            }),
        ..Default::default()
    };
    eframe::run_native(
        "PrintMarten",
        options,
        Box::new(move |cc| {
            Ok(Box::new(ui::App::new(
                cc,
                args.into_iter().map(Into::into).collect(),
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}
