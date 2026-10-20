//! Thin wrapper around `NSPasteboard` (read a snapshot, write a stored payload).

use macclip::privacy::Snapshot;
use macclip::store::Payload;
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSImage, NSPasteboard,
    NSPasteboardTypeHTML, NSPasteboardTypePNG, NSPasteboardTypeRTF, NSPasteboardTypeString,
    NSPasteboardTypeTIFF,
};
use objc2_foundation::{NSData, NSDictionary, NSPoint, NSRect, NSSize};

/// Longest edge of stored thumbnails, in pixels.
const THUMB_EDGE: f64 = 160.0;

pub fn change_count() -> isize {
    NSPasteboard::generalPasteboard().changeCount()
}

fn data_vec(d: Option<Retained<NSData>>) -> Option<Vec<u8>> {
    d.map(|d| d.to_vec()).filter(|v| !v.is_empty())
}

/// Read every representation we care about from the general pasteboard.
pub fn read_snapshot() -> Snapshot {
    let pb = NSPasteboard::generalPasteboard();
    let types: Vec<String> = pb
        .types()
        .map(|a| a.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let (text, rtf, html) = unsafe {
        (
            pb.stringForType(NSPasteboardTypeString)
                .map(|s| s.to_string()),
            data_vec(pb.dataForType(NSPasteboardTypeRTF)),
            data_vec(pb.dataForType(NSPasteboardTypeHTML)),
        )
    };

    // Images are only read when there is no text, which avoids needless
    // conversion work for web-page copies that carry both.
    let (mut image_png, mut thumb_png) = (None, None);
    if text.is_none() {
        let png = unsafe { data_vec(pb.dataForType(NSPasteboardTypePNG)) };
        let png = png.or_else(|| {
            let tiff = unsafe { pb.dataForType(NSPasteboardTypeTIFF) }?;
            to_png(&tiff)
        });
        if let Some(png) = png {
            thumb_png = make_thumbnail(&png);
            image_png = Some(png);
        }
    }
    Snapshot {
        types,
        text,
        rtf,
        html,
        image_png,
        thumb_png,
    }
}

/// Replace the pasteboard contents with a stored payload; returns the new
/// change count so the monitor can ignore our own write.
pub fn write_payload(p: &Payload) -> isize {
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    unsafe {
        if let Some(png) = &p.image_png {
            pb.setData_forType(Some(&NSData::with_bytes(png)), NSPasteboardTypePNG);
        }
        if let Some(t) = &p.text {
            pb.setString_forType(
                &objc2_foundation::NSString::from_str(t),
                NSPasteboardTypeString,
            );
        }
        if let Some(r) = &p.rtf {
            pb.setData_forType(Some(&NSData::with_bytes(r)), NSPasteboardTypeRTF);
        }
        if let Some(h) = &p.html {
            pb.setData_forType(Some(&NSData::with_bytes(h)), NSPasteboardTypeHTML);
        }
    }
    pb.changeCount()
}

fn to_png(image_data: &NSData) -> Option<Vec<u8>> {
    let rep = NSBitmapImageRep::imageRepWithData(image_data)?;
    let props = NSDictionary::new();
    let out =
        unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &props) }?;
    Some(out.to_vec())
}

/// Downscale `png` so its longest edge is at most `THUMB_EDGE`.
fn make_thumbnail(png: &[u8]) -> Option<Vec<u8>> {
    let data = NSData::with_bytes(png);
    let src = NSImage::initWithData(NSImage::alloc(), &data)?;
    let size = src.size();
    if size.width < 1.0 || size.height < 1.0 {
        return None;
    }
    let scale = (THUMB_EDGE / size.width.max(size.height)).min(1.0);
    let tsize = NSSize::new(
        (size.width * scale).max(1.0).round(),
        (size.height * scale).max(1.0).round(),
    );
    let dst = NSImage::initWithSize(NSImage::alloc(), tsize);
    #[allow(deprecated)]
    {
        dst.lockFocus();
        src.drawInRect_fromRect_operation_fraction(
            NSRect::new(NSPoint::new(0.0, 0.0), tsize),
            NSRect::ZERO,
            NSCompositingOperation::Copy,
            1.0,
        );
        dst.unlockFocus();
    }
    let tiff = dst.TIFFRepresentation()?;
    to_png(&tiff)
}
