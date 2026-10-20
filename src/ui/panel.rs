//! The floating history panel: a non-activating NSPanel that never steals
//! focus from the app the user is typing in.

use macclip::store::{Entry, Kind};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSBackingStoreType, NSBox, NSBoxType, NSColor, NSEvent, NSFont,
    NSImage, NSImageScaling, NSImageView, NSLineBreakMode, NSPanel, NSTextAlignment, NSTextField,
    NSTitlePosition, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

pub const WIDTH: f64 = 320.0;
pub const HEADER_H: f64 = 28.0;
pub const ROW_H: f64 = 32.0;
pub const MAX_VISIBLE: usize = 8;
const CORNER: f64 = 10.0;
/// NSPopUpMenuWindowLevel
const WINDOW_LEVEL: isize = 101;

define_class!(
    // SAFETY: NSPanel has no subclassing requirements beyond calling super.
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    #[name = "MacClipPanel"]
    pub struct ClipPanel;

    impl ClipPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool {
            false
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            crate::app::on_key(event);
        }

        #[unsafe(method(cancelOperation:))]
        fn cancel_operation(&self, _sender: Option<&AnyObject>) {
            crate::app::on_escape();
        }

        #[unsafe(method(resignKeyWindow))]
        fn resign_key_window(&self) {
            // SAFETY: calling the superclass implementation with its own signature.
            let _: () = unsafe { msg_send![super(self), resignKeyWindow] };
            crate::app::on_resign_key();
        }
    }
);

define_class!(
    // Transparent view above the rows that turns clicks and scrolls into app events.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "MacClipOverlay"]
    pub struct Overlay;

    impl Overlay {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let p = self.convertPoint_fromView(event.locationInWindow(), None);
            let from_top = self.bounds().size.height - p.y;
            if from_top >= 0.0 {
                crate::app::on_click((from_top / ROW_H) as usize);
            }
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            crate::app::on_scroll(event.scrollingDeltaY());
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }
    }
);

/// What to draw.
pub struct View<'a> {
    pub entries: &'a [Entry],
    pub query: &'a str,
    pub selected: usize,
    pub top: usize,
    pub trusted: bool,
    pub now_ms: i64,
}

pub struct Panel {
    window: Retained<ClipPanel>,
    root: Retained<NSVisualEffectView>,
    title: Retained<NSTextField>,
    hint: Retained<NSTextField>,
    list: Retained<NSView>,
    overlay: Retained<Overlay>,
}

impl Panel {
    pub fn new(mtm: MainThreadMarker) -> Panel {
        let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, HEADER_H + ROW_H));
        let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
        let this = mtm.alloc::<ClipPanel>().set_ivars(());
        // SAFETY: designated initializer of NSPanel/NSWindow with valid arguments.
        let window: Retained<ClipPanel> = unsafe {
            msg_send![super(this), initWithContentRect: rect, styleMask: style, backing: NSBackingStoreType::Buffered, defer: false]
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setFloatingPanel(true);
        window.setLevel(WINDOW_LEVEL);
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        window.setHasShadow(true);
        window.setHidesOnDeactivate(false);
        window.setBecomesKeyOnlyIfNeeded(false);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        window.setMovableByWindowBackground(false);

        let root = NSVisualEffectView::initWithFrame(mtm.alloc::<NSVisualEffectView>(), rect);
        root.setMaterial(NSVisualEffectMaterial::Popover);
        root.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        root.setState(NSVisualEffectState::Active);
        root.setWantsLayer(true);
        // SAFETY: CALayer responds to these; the layer exists after setWantsLayer.
        unsafe {
            let layer: Option<Retained<AnyObject>> = msg_send![&*root, layer];
            if let Some(layer) = layer {
                let _: () = msg_send![&*layer, setCornerRadius: CORNER];
                let _: () = msg_send![&*layer, setMasksToBounds: true];
            }
        }
        window.setContentView(Some(&root));

        let title = NSTextField::labelWithString(&NSString::from_str("MacClip"), mtm);
        title.setFont(Some(&NSFont::systemFontOfSize_weight(11.0, 0.3)));
        title.setTextColor(Some(&NSColor::secondaryLabelColor()));
        let hint = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        hint.setFont(Some(&NSFont::systemFontOfSize(11.0)));
        hint.setAlignment(NSTextAlignment::Right);
        let list = NSView::initWithFrame(mtm.alloc::<NSView>(), rect);
        let overlay = Overlay::alloc_overlay(mtm, rect);

        root.addSubview(&list);
        root.addSubview(&title);
        root.addSubview(&hint);
        root.addSubview(&overlay);
        let _ = NSAutoresizingMaskOptions::empty();

        Panel {
            window,
            root,
            title,
            hint,
            list,
            overlay,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.window.isVisible()
    }

    pub fn is_key(&self) -> bool {
        self.window.isKeyWindow()
    }

    /// Render the panel contents to PNG without any screen-capture permission.
    pub fn snapshot_png(&self) -> Option<Vec<u8>> {
        let bounds = self.root.bounds();
        let rep = self.root.bitmapImageRepForCachingDisplayInRect(bounds)?;
        self.root.cacheDisplayInRect_toBitmapImageRep(bounds, &rep);
        let props = objc2_foundation::NSDictionary::new();
        let data = unsafe {
            rep.representationUsingType_properties(
                objc2_app_kit::NSBitmapImageFileType::PNG,
                &props,
            )
        }?;
        Some(data.to_vec())
    }

    pub fn size(&self) -> NSSize {
        self.window.frame().size
    }

    /// Redraw, keeping the panel's top edge fixed while its height changes.
    pub fn render(&self, mtm: MainThreadMarker, v: &View) {
        let visible: Vec<&Entry> = v.entries.iter().skip(v.top).take(MAX_VISIBLE).collect();
        let rows = visible.len().max(1);
        let list_h = rows as f64 * ROW_H;
        let total_h = list_h + HEADER_H;

        let f = self.window.frame();
        let new_frame = NSRect::new(
            NSPoint::new(f.origin.x, f.origin.y + f.size.height - total_h),
            NSSize::new(WIDTH, total_h),
        );
        self.window.setFrame_display(new_frame, true);
        self.root.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(WIDTH, total_h),
        ));

        let list_rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, list_h));
        self.list.setFrame(list_rect);
        self.overlay.setFrame(list_rect);

        self.title.setFrame(NSRect::new(
            NSPoint::new(12.0, list_h + 5.0),
            NSSize::new(70.0, 16.0),
        ));
        self.hint.setFrame(NSRect::new(
            NSPoint::new(84.0, list_h + 5.0),
            NSSize::new(WIDTH - 96.0, 16.0),
        ));
        if v.query.is_empty() {
            let text = if v.trusted {
                "Type to search"
            } else {
                "Enable Accessibility to auto-paste"
            };
            self.hint.setStringValue(&NSString::from_str(text));
            self.hint.setTextColor(Some(&NSColor::tertiaryLabelColor()));
        } else {
            self.hint
                .setStringValue(&NSString::from_str(&format!("Search: {}", v.query)));
            self.hint
                .setTextColor(Some(&NSColor::secondaryLabelColor()));
        }

        for sub in self.list.subviews().iter() {
            sub.removeFromSuperview();
        }

        if visible.is_empty() {
            let msg = if v.query.is_empty() {
                "Nothing copied yet"
            } else {
                "No matches"
            };
            let l = NSTextField::labelWithString(&NSString::from_str(msg), mtm);
            l.setFont(Some(&NSFont::systemFontOfSize(12.0)));
            l.setTextColor(Some(&NSColor::tertiaryLabelColor()));
            l.setAlignment(NSTextAlignment::Center);
            l.setFrame(NSRect::new(
                NSPoint::new(0.0, ROW_H / 2.0 - 8.0),
                NSSize::new(WIDTH, 16.0),
            ));
            self.list.addSubview(&l);
            return;
        }

        for (row, entry) in visible.iter().enumerate() {
            let abs = v.top + row;
            let y = list_h - (row as f64 + 1.0) * ROW_H;
            self.add_row(mtm, abs, y, entry, abs == v.selected, v.now_ms);
        }
    }

    fn add_row(
        &self,
        mtm: MainThreadMarker,
        abs: usize,
        y: f64,
        e: &Entry,
        selected: bool,
        now_ms: i64,
    ) {
        // Quiet palette: selection is a faint tint, text colours never change.
        if selected {
            let b = NSBox::new(mtm);
            b.setBoxType(NSBoxType::Custom);
            b.setTitlePosition(NSTitlePosition::NoTitle);
            b.setBorderWidth(0.0);
            b.setCornerRadius(6.0);
            b.setFillColor(&NSColor::labelColor().colorWithAlphaComponent(0.09));
            b.setFrame(NSRect::new(
                NSPoint::new(4.0, y + 1.0),
                NSSize::new(WIDTH - 8.0, ROW_H - 2.0),
            ));
            self.list.addSubview(&b);
        }

        if abs < 9 {
            let n = NSTextField::labelWithString(&NSString::from_str(&(abs + 1).to_string()), mtm);
            n.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
                10.0, 0.0,
            )));
            n.setTextColor(Some(&NSColor::tertiaryLabelColor()));
            n.setAlignment(NSTextAlignment::Center);
            n.setFrame(NSRect::new(
                NSPoint::new(8.0, y + ROW_H / 2.0 - 7.0),
                NSSize::new(14.0, 14.0),
            ));
            self.list.addSubview(&n);
        }

        let x0 = 28.0;
        let mut text_x = x0;
        let title_text;
        match e.kind {
            Kind::Image => {
                let img = e
                    .thumb_path
                    .as_ref()
                    .and_then(|p| {
                        NSImage::initWithContentsOfFile(
                            NSImage::alloc(),
                            &NSString::from_str(&p.to_string_lossy()),
                        )
                    })
                    .or_else(|| {
                        NSImage::imageWithSystemSymbolName_accessibilityDescription(
                            &NSString::from_str("photo"),
                            None,
                        )
                    });
                if let Some(img) = img {
                    let iv = NSImageView::imageViewWithImage(&img, mtm);
                    iv.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
                    iv.setFrame(NSRect::new(
                        NSPoint::new(x0, y + 5.0),
                        NSSize::new(32.0, ROW_H - 10.0),
                    ));
                    self.list.addSubview(&iv);
                }
                text_x = x0 + 40.0;
                title_text = "Image".to_string();
            }
            Kind::Text => {
                title_text = e.preview.clone();
            }
        }

        // Relative time, right aligned and faint.
        let time_w = 44.0;
        let meta = NSTextField::labelWithString(
            &NSString::from_str(&relative_time(now_ms, e.created_at)),
            mtm,
        );
        meta.setFont(Some(&NSFont::systemFontOfSize(10.0)));
        meta.setTextColor(Some(&NSColor::tertiaryLabelColor()));
        meta.setAlignment(NSTextAlignment::Right);
        meta.setFrame(NSRect::new(
            NSPoint::new(WIDTH - time_w - 10.0, y + ROW_H / 2.0 - 7.0),
            NSSize::new(time_w, 14.0),
        ));
        self.list.addSubview(&meta);

        // Single-line, truncated text.
        let title = NSTextField::labelWithString(&NSString::from_str(&title_text), mtm);
        title.setFont(Some(&NSFont::systemFontOfSize(12.0)));
        title.setTextColor(Some(&NSColor::labelColor()));
        title.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        title.setFrame(NSRect::new(
            NSPoint::new(text_x, y + ROW_H / 2.0 - 8.0),
            NSSize::new(WIDTH - text_x - time_w - 16.0, 16.0),
        ));
        self.list.addSubview(&title);
    }

    /// Show with its top-left corner at `top_left` (screen coordinates).
    pub fn show(&self, top_left: NSPoint) {
        let h = self.window.frame().size.height;
        self.window
            .setFrameOrigin(NSPoint::new(top_left.x, top_left.y - h));
        self.window.makeKeyAndOrderFront(None);
    }

    pub fn hide(&self) {
        self.window.orderOut(None);
    }
}

impl Overlay {
    fn alloc_overlay(mtm: MainThreadMarker, frame: NSRect) -> Retained<Overlay> {
        let this = mtm.alloc::<Overlay>().set_ivars(());
        // SAFETY: NSView's designated initializer.
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

pub fn relative_time(now_ms: i64, then_ms: i64) -> String {
    let s = ((now_ms - then_ms).max(0)) / 1000;
    match s {
        0..=4 => "just now".into(),
        5..=59 => format!("{s}s ago"),
        60..=3599 => format!("{}m ago", s / 60),
        3600..=86_399 => format!("{}h ago", s / 3600),
        _ => format!("{}d ago", s / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::relative_time;

    #[test]
    fn formats_relative_time() {
        assert_eq!(relative_time(10_000, 9_000), "just now");
        assert_eq!(relative_time(40_000, 10_000), "30s ago");
        assert_eq!(relative_time(600_000, 0), "10m ago");
        assert_eq!(relative_time(7_200_000, 0), "2h ago");
        assert_eq!(relative_time(3 * 86_400_000, 0), "3d ago");
    }
}
