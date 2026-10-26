use super::*;
use ::windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        Storage::Xps::*,
        System::{Com::*, Memory::*},
        UI::{Controls::Dialogs::*, WindowsAndMessaging::GetForegroundWindow},
    },
    core::w,
};
use std::mem::{size_of, zeroed};

pub struct Session {
    dialog: PRINTDLGEXW,
    ranges: Vec<PRINTPAGERANGE>,
    com_initialized: bool,
}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            if !self.dialog.hDC.is_invalid() {
                let _ = DeleteDC(self.dialog.hDC);
            }
            if !self.dialog.hDevMode.is_invalid() {
                let _ = GlobalFree(Some(self.dialog.hDevMode));
            }
            if !self.dialog.hDevNames.is_invalid() {
                let _ = GlobalFree(Some(self.dialog.hDevNames));
            }
            if self.com_initialized {
                CoUninitialize();
            }
        }
    }
}
fn um(value: i32, dpi: i32) -> i64 {
    (value as f64 * 25400.0 / dpi as f64).round() as i64
}
pub fn prepare(settings: &Settings, pages: usize) -> Result<Option<Prepared>> {
    unsafe {
        // GUI frameworks may already have initialized the STA; S_FALSE is valid.
        let com_initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let mut session = Session {
            dialog: PRINTDLGEXW {
                lStructSize: size_of::<PRINTDLGEXW>() as u32,
                hwndOwner: GetForegroundWindow(),
                Flags: PD_RETURNDEFAULT,
                nStartPage: START_PAGE_GENERAL,
                ..zeroed()
            },
            ranges: vec![],
            com_initialized,
        };
        PrintDlgExW(&mut session.dialog)?;
        anyhow::ensure!(
            !session.dialog.hDevMode.is_invalid(),
            "No default printer is configured"
        );
        let mode = GlobalLock(session.dialog.hDevMode) as *mut DEVMODEW;
        anyhow::ensure!(!mode.is_null(), "Cannot access printer settings");
        let landscape = settings.paper.w > settings.paper.h;
        let s = if landscape {
            settings.paper.rotated()
        } else {
            settings.paper
        };
        (*mode).dmFields |= DM_PAPERSIZE | DM_ORIENTATION | DM_SCALE;
        (*mode).dmFields &= !(DM_FORMNAME | DM_PAPERWIDTH | DM_PAPERLENGTH);
        let paper_id = match (s.w, s.h) {
            (297000, 420000) => DMPAPER_A3,
            (210000, 297000) => DMPAPER_A4,
            (148000, 210000) => DMPAPER_A5,
            (105000, 148000) => DMPAPER_A6,
            (215900, 279400) => DMPAPER_LETTER,
            (215900, 355600) => DMPAPER_LEGAL,
            _ => 0,
        };
        (*mode).Anonymous1.Anonymous1.dmPaperSize = paper_id as i16;
        if paper_id == 0 {
            (*mode).dmFields |= DM_PAPERWIDTH | DM_PAPERLENGTH;
            (*mode).Anonymous1.Anonymous1.dmPaperWidth = (s.w / 100) as i16;
            (*mode).Anonymous1.Anonymous1.dmPaperLength = (s.h / 100) as i16;
        }
        (*mode).Anonymous1.Anonymous1.dmOrientation = if landscape {
            DMORIENT_LANDSCAPE as i16
        } else {
            DMORIENT_PORTRAIT as i16
        };
        (*mode).Anonymous1.Anonymous1.dmScale = 100;
        let _ = GlobalUnlock(session.dialog.hDevMode);
        let mut ranges = vec![PRINTPAGERANGE::default(); 64];
        session.dialog.Flags =
            PD_RETURNDC | PD_USEDEVMODECOPIESANDCOLLATE | PD_NOSELECTION | PD_NOCURRENTPAGE;
        session.dialog.nMinPage = 1;
        session.dialog.nMaxPage = pages.max(1) as u32;
        session.dialog.nMaxPageRanges = ranges.len() as u32;
        session.dialog.lpPageRanges = ranges.as_mut_ptr();
        PrintDlgExW(&mut session.dialog)?;
        if session.dialog.dwResultAction != PD_RESULT_PRINT {
            return Ok(None);
        }
        let dc = session.dialog.hDC;
        anyhow::ensure!(
            !dc.is_invalid(),
            "The printer did not return a drawing context"
        );
        let (dx, dy) = (
            GetDeviceCaps(Some(dc), LOGPIXELSX),
            GetDeviceCaps(Some(dc), LOGPIXELSY),
        );
        anyhow::ensure!(dx > 0 && dy > 0, "Invalid printer resolution");
        let (w, h) = (
            GetDeviceCaps(Some(dc), PHYSICALWIDTH),
            GetDeviceCaps(Some(dc), PHYSICALHEIGHT),
        );
        let (x, y) = (
            GetDeviceCaps(Some(dc), PHYSICALOFFSETX),
            GetDeviceCaps(Some(dc), PHYSICALOFFSETY),
        );
        let (rw, rh) = (
            GetDeviceCaps(Some(dc), HORZRES),
            GetDeviceCaps(Some(dc), VERTRES),
        );
        let mode = GlobalLock(session.dialog.hDevMode) as *mut DEVMODEW;
        let scale = if mode.is_null() {
            100.0
        } else {
            let scale = if (*mode).dmFields.contains(DM_SCALE) {
                (*mode).Anonymous1.Anonymous1.dmScale as f64
            } else {
                100.0
            };
            let _ = GlobalUnlock(session.dialog.hDevMode);
            scale
        };
        // Save page ranges as owned data; PRINTDLGEX's temporary buffer must not escape.
        session.ranges = if session.dialog.Flags.contains(PD_PAGENUMS) {
            ranges[..session.dialog.nPageRanges as usize].to_vec()
        } else {
            vec![]
        };
        session.dialog.lpPageRanges = std::ptr::null_mut();
        session.dialog.nPageRanges = 0;
        let geometry = Geometry {
            paper: Size {
                w: um(w, dx),
                h: um(h, dy),
            },
            hardware: Insets {
                left: um(x, dx),
                top: um(y, dy),
                right: um(w - x - rw, dx).max(0),
                bottom: um(h - y - rh, dy).max(0),
            },
            scale_percent: scale,
            pages_per_sheet: 1,
        };
        Ok(Some(Prepared { geometry, session }))
    }
}
impl Session {
    pub fn submit(self, layout: &Layout, sources: &[Source]) -> Result<()> {
        unsafe {
            let dc = self.dialog.hDC;
            let ranges = &self.ranges;
            let selected: Vec<_> = (0..layout.pages.len())
                .filter(|i| {
                    ranges.is_empty()
                        || ranges
                            .iter()
                            .any(|r| *i + 1 >= r.nFromPage as usize && *i < r.nToPage as usize)
                })
                .collect();
            anyhow::ensure!(
                !selected.is_empty(),
                "The selected page range contains no pages"
            );
            let info = DOCINFOW {
                cbSize: size_of::<DOCINFOW>() as i32,
                lpszDocName: w!("PrintMarten photos"),
                ..zeroed()
            };
            anyhow::ensure!(StartDocW(dc, &info) > 0, "Cannot start the print job");
            let result = (|| -> Result<()> {
                for index in selected {
                    let rgb = crate::output::raster_page(layout, index, sources, 300.0)?;
                    let mut bgra =
                        Vec::with_capacity(rgb.width() as usize * rgb.height() as usize * 4);
                    for p in rgb.pixels() {
                        bgra.extend_from_slice(&[p[2], p[1], p[0], 0]);
                    }
                    let bitmap = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: rgb.width() as i32,
                            biHeight: -(rgb.height() as i32),
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB.0,
                            ..zeroed()
                        },
                        ..zeroed()
                    };
                    anyhow::ensure!(StartPage(dc) > 0, "Cannot start a printed page");
                    let x = GetDeviceCaps(Some(dc), PHYSICALOFFSETX);
                    let y = GetDeviceCaps(Some(dc), PHYSICALOFFSETY);
                    let w = GetDeviceCaps(Some(dc), PHYSICALWIDTH);
                    let h = GetDeviceCaps(Some(dc), PHYSICALHEIGHT);
                    let _ = SetStretchBltMode(dc, HALFTONE);
                    let lines = StretchDIBits(
                        dc,
                        -x,
                        -y,
                        w,
                        h,
                        0,
                        0,
                        rgb.width() as i32,
                        rgb.height() as i32,
                        Some(bgra.as_ptr().cast()),
                        &bitmap,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                    anyhow::ensure!(lines > 0, "The printer rejected the image data");
                    anyhow::ensure!(EndPage(dc) > 0, "Cannot finish the printed page");
                }
                anyhow::ensure!(EndDoc(dc) > 0, "Cannot finish the print job");
                Ok(())
            })();
            if result.is_err() {
                let _ = AbortDoc(dc);
            }
            result
        }
    }
}
