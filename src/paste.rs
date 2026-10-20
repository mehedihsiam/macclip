//! Synthesize ⌘V into the frontmost application.

use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
};

/// `kVK_ANSI_V`
const KEY_V: u16 = 9;

/// Post ⌘V key-down/up to the HID event tap. Requires Accessibility trust.
pub fn send_cmd_v() {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState);
    for down in [true, false] {
        if let Some(ev) = CGEvent::new_keyboard_event(source.as_deref(), KEY_V, down) {
            CGEvent::set_flags(Some(&ev), CGEventFlags::MaskCommand);
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&ev));
        }
    }
}

/// Show a notification without needing any notification entitlement.
pub fn notify(title: &str, body: &str) {
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        esc(body),
        esc(title)
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .spawn();
}
