use super::*;
use anyhow::Context;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

// A helper process owns GTK's main context and the live GtkPrintSetup. This keeps
// GTK's event loop separate from eframe/winit, on both X11 and Wayland.
pub struct Session {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[derive(Serialize, Deserialize)]
struct Request {
    settings: Settings,
    pages: usize,
}
#[derive(Serialize, Deserialize)]
struct Reply {
    geometry: Option<Geometry>,
    error: Option<String>,
}
pub fn prepare(settings: &Settings, pages: usize) -> Result<Option<Prepared>> {
    let mut child = Command::new(std::env::current_exe()?)
        .arg("--print-helper")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let input = child.stdin.take().context("Missing helper input")?;
    let output = BufReader::new(child.stdout.take().context("Missing helper output")?);
    let mut session = Session {
        child,
        input,
        output,
    };
    serde_json::to_writer(
        &mut session.input,
        &Request {
            settings: settings.clone(),
            pages,
        },
    )?;
    writeln!(session.input)?;
    session.input.flush()?;
    let mut line = String::new();
    session.output.read_line(&mut line)?;
    let reply: Reply = serde_json::from_str(&line)
        .context("The print dialog helper did not return valid settings")?;
    if let Some(error) = reply.error {
        anyhow::bail!(error);
    }
    Ok(reply
        .geometry
        .map(|geometry| Prepared { geometry, session }))
}
impl Session {
    pub fn submit(mut self, layout: &Layout, sources: &[Source]) -> Result<()> {
        let file = tempfile::Builder::new().suffix(".pdf").tempfile()?;
        crate::output::write_pdf(layout, sources, file.path())?;
        serde_json::to_writer(&mut self.input, &file.path())?;
        writeln!(self.input)?;
        self.input.flush()?;
        let mut line = String::new();
        self.output.read_line(&mut line)?;
        let error: Option<String> = serde_json::from_str(&line)
            .context("The print helper exited before completing submission")?;
        if let Some(error) = error {
            anyhow::bail!(error);
        }
        Ok(())
    }
}

fn default_borderless_setup(paper: Size) -> Option<gtk4::PageSetup> {
    use gtk4::{glib, prelude::*};
    use std::sync::{Arc, Mutex};
    use std::{cell::Cell, rc::Rc, time::Duration};
    // Keep the printer's actual paper identity (including its borderless suffix),
    // rather than manufacturing zero margins for a paper the driver cannot print.
    let selected = Arc::new(Mutex::new(None));
    let result = selected.clone();
    gtk4::enumerate_printers(
        move |printer| {
            if !printer.is_default() {
                return false;
            }
            if !printer.has_details() {
                let wait = glib::MainLoop::new(None, false);
                let done = Rc::new(Cell::new(false));
                let finished = done.clone();
                let details_wait = wait.clone();
                let handler = printer.connect_details_acquired(move |_, _| {
                    finished.set(true);
                    details_wait.quit();
                });
                let expired = Rc::new(Cell::new(false));
                let timeout_expired = expired.clone();
                let timeout_wait = wait.clone();
                let timeout = glib::timeout_add_local_once(Duration::from_secs(3), move || {
                    timeout_expired.set(true);
                    timeout_wait.quit();
                });
                printer.request_details();
                if !done.get() {
                    wait.run();
                }
                if !expired.get() {
                    timeout.remove();
                }
                printer.disconnect(handler);
            }
            for setup in printer.list_papers() {
                let um = |value: f64| (value * 1000.0).round() as i64;
                if (um(setup.paper_width(gtk4::Unit::Mm)) - paper.w).abs() <= 100
                    && (um(setup.paper_height(gtk4::Unit::Mm)) - paper.h).abs() <= 100
                    && [
                        setup.top_margin(gtk4::Unit::Mm),
                        setup.bottom_margin(gtk4::Unit::Mm),
                        setup.left_margin(gtk4::Unit::Mm),
                        setup.right_margin(gtk4::Unit::Mm),
                    ]
                    .iter()
                    .all(|margin| *margin <= 0.0)
                {
                    *selected.lock().unwrap() = Some(setup.to_gvariant());
                    break;
                }
            }
            true
        },
        true,
    );
    result
        .lock()
        .unwrap()
        .as_ref()
        .map(gtk4::PageSetup::from_gvariant)
}

pub fn helper() -> Result<()> {
    use gtk4::{gio, glib};
    let mut input = BufReader::new(std::io::stdin());
    let mut line = String::new();
    input.read_line(&mut line)?;
    let request: Request = serde_json::from_str(&line)?;
    gtk4::init()?;
    let context = glib::MainContext::default();
    context.block_on(async {
        let landscape = request.settings.paper.w > request.settings.paper.h;
        let paper = if landscape {
            request.settings.paper.rotated()
        } else {
            request.settings.paper
        };
        let standard = match (paper.w, paper.h) {
            (297000, 420000) => Some("iso_a3"),
            (210000, 297000) => Some("iso_a4"),
            (148000, 210000) => Some("iso_a5"),
            (105000, 148000) => Some("iso_a6"),
            (215900, 279400) => Some("na_letter"),
            (215900, 355600) => Some("na_legal"),
            _ => None,
        };
        let size = if let Some(name) = standard {
            gtk4::PaperSize::new(Some(name))
        } else {
            gtk4::PaperSize::new_custom(
                "printmarten",
                "PrintMarten paper",
                paper.w as f64 / 1000.0,
                paper.h as f64 / 1000.0,
                gtk4::Unit::Mm,
            )
        };
        let setup = if request.settings.borderless {
            default_borderless_setup(paper)
        } else {
            None
        }
        .unwrap_or_else(|| {
            let setup = gtk4::PageSetup::new();
            setup.set_paper_size_and_default_margins(&size);
            setup
        });
        setup.set_orientation(if landscape {
            gtk4::PageOrientation::Landscape
        } else {
            gtk4::PageOrientation::Portrait
        });
        let settings = gtk4::PrintSettings::new();
        if request.settings.photo_paper {
            settings.set_media_type("photographic");
        }
        settings.set_scale(100.0);
        settings.set_number_up(1);
        let dialog = gtk4::PrintDialog::new();
        dialog.set_page_setup(&setup);
        dialog.set_print_settings(&settings);
        dialog.set_title("Print PrintMarten photos");
        let selected = match dialog.setup_future(None::<&gtk4::Window>).await {
            Ok(selected) => selected,
            Err(error) => {
                let dismissed = error.matches(gtk4::DialogError::Dismissed);
                println!(
                    "{}",
                    serde_json::to_string(&Reply {
                        geometry: None,
                        error: if dismissed {
                            None
                        } else {
                            Some(error.to_string())
                        }
                    })?
                );
                return Ok(());
            }
        };
        let setup = selected.page_setup();
        let settings = selected.print_settings();
        let um = |v: f64| (v * 1000.0).round() as i64;
        let geometry = Geometry {
            paper: Size {
                w: um(setup.paper_width(gtk4::Unit::Mm)),
                h: um(setup.paper_height(gtk4::Unit::Mm)),
            },
            hardware: Insets {
                left: um(setup.left_margin(gtk4::Unit::Mm)).max(0),
                right: um(setup.right_margin(gtk4::Unit::Mm)).max(0),
                top: um(setup.top_margin(gtk4::Unit::Mm)).max(0),
                bottom: um(setup.bottom_margin(gtk4::Unit::Mm)).max(0),
            },
            scale_percent: settings.scale(),
            pages_per_sheet: settings.number_up() as u32,
        };
        println!(
            "{}",
            serde_json::to_string(&Reply {
                geometry: Some(geometry),
                error: None
            })?
        );
        std::io::stdout().flush()?;
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let path: std::path::PathBuf = serde_json::from_str(&line)?;
        let result = dialog
            .print_file_future(
                None::<&gtk4::Window>,
                Some(&selected),
                &gio::File::for_path(path),
            )
            .await;
        println!(
            "{}",
            serde_json::to_string(&result.err().map(|e| e.to_string()))?
        );
        Ok(())
    })
}
