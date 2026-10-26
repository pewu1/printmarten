use super::*;
use anyhow::Context;
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject},
};
use objc2_foundation::{NSData, NSRect, NSSize, NSString};

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}
#[link(name = "PDFKit", kind = "framework")]
unsafe extern "C" {}

pub struct Session {
    info: Retained<AnyObject>,
}
fn class(name: &std::ffi::CStr) -> Result<&'static AnyClass> {
    AnyClass::get(name).context("Required macOS framework class is unavailable")
}
fn um(points: f64) -> i64 {
    (points * 25400.0 / 72.0).round() as i64
}

pub fn prepare(settings: &Settings, _pages: usize) -> Result<Option<Prepared>> {
    // Called on the GUI thread. NSPrintPanel edits this private print-info copy;
    // retaining it preserves destination, page ranges, copies, and driver options.
    unsafe {
        let shared: *mut AnyObject = msg_send![class(c"NSPrintInfo")?, sharedPrintInfo];
        let info: *mut AnyObject = msg_send![shared, copy];
        let info = Retained::from_raw(info).context("Cannot create print settings")?;
        let landscape = settings.paper.w > settings.paper.h;
        let canonical = if landscape {
            settings.paper.rotated()
        } else {
            settings.paper
        };
        let size = NSSize::new(
            canonical.w as f64 * 72.0 / 25400.0,
            canonical.h as f64 * 72.0 / 25400.0,
        );
        let _: () = msg_send![&*info,setPaperSize:size];
        let _: () = msg_send![&*info,setOrientation:if landscape {1_usize}else{0_usize}];
        let panel: *mut AnyObject = msg_send![class(c"NSPrintPanel")?, printPanel];
        // The app owns the sheet preview; the panel edits copies, page ranges,
        // paper, orientation, and scaling without submitting a job.
        let _: () = msg_send![panel,setOptions: (1_usize|2|4|8|16)];
        let result: isize = msg_send![panel,runModalWithPrintInfo:&*info];
        if result != 1 {
            return Ok(None);
        }
        let size: NSSize = msg_send![&*info, paperSize];
        let bounds: NSRect = msg_send![&*info, imageablePageBounds];
        let dict: *mut AnyObject = msg_send![&*info, dictionary];
        let read = |key: &str, default: f64| -> f64 {
            let key = NSString::from_str(key);
            let value: *mut AnyObject = msg_send![dict,objectForKey:&*key];
            if value.is_null() {
                default
            } else {
                msg_send![value, doubleValue]
            }
        };
        let geometry = Geometry {
            paper: Size {
                w: um(size.width),
                h: um(size.height),
            },
            hardware: Insets {
                left: um(bounds.origin.x).max(0),
                bottom: um(bounds.origin.y).max(0),
                right: um(size.width - bounds.origin.x - bounds.size.width).max(0),
                top: um(size.height - bounds.origin.y - bounds.size.height).max(0),
            },
            scale_percent: read("NSPrintScalingFactor", 1.0) * 100.0,
            pages_per_sheet: read("NSPrintPagesPerSheet", 1.0) as u32,
        };
        // The PDF already includes physical page coordinates and margins.
        let _: () = msg_send![&*info,setTopMargin:0.0_f64];
        let _: () = msg_send![&*info,setBottomMargin:0.0_f64];
        let _: () = msg_send![&*info,setLeftMargin:0.0_f64];
        let _: () = msg_send![&*info,setRightMargin:0.0_f64];
        let _: () = msg_send![&*info,setHorizontallyCentered:false];
        let _: () = msg_send![&*info,setVerticallyCentered:false];
        Ok(Some(Prepared {
            geometry,
            session: Session { info },
        }))
    }
}
impl Session {
    pub fn submit(self, layout: &Layout, sources: &[Source]) -> Result<()> {
        let file = tempfile::Builder::new().suffix(".pdf").tempfile()?;
        crate::output::write_pdf(layout, sources, file.path())?;
        let bytes = std::fs::read(file.path())?;
        unsafe {
            let data = NSData::with_bytes(&bytes);
            let allocated: *mut AnyObject = msg_send![class(c"PDFDocument")?, alloc];
            let doc: *mut AnyObject = msg_send![allocated,initWithData:&*data];
            let doc = Retained::from_raw(doc).context("Cannot open print document")?;
            let operation: *mut AnyObject = msg_send![&*doc,printOperationForPrintInfo:&*self.info,scalingMode:0_isize,autoRotate:false];
            let operation = Retained::retain(operation).context("Cannot create print operation")?;
            let _: () = msg_send![&*operation,setShowsPrintPanel:false];
            let success: bool = msg_send![&*operation, runOperation];
            anyhow::ensure!(success, "The print operation was cancelled or failed");
        }
        Ok(())
    }
}
