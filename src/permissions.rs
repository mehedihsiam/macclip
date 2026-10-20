//! Accessibility permission (needed only to synthesize ⌘V for auto-paste).

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSDictionary, NSNumber, NSString};
use std::ffi::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
}

pub const SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

/// Is MacClip allowed to control the computer (post key events)?
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Ask macOS to show its Accessibility prompt (adds MacClip to the list).
pub fn prompt() -> bool {
    let key = NSString::from_str("AXTrustedCheckOptionPrompt");
    let val: Retained<AnyObject> = NSNumber::new_bool(true).into();
    let dict: Retained<NSDictionary<NSString, AnyObject>> =
        NSDictionary::from_retained_objects(&[&*key], &[val]);
    unsafe { AXIsProcessTrustedWithOptions(Retained::as_ptr(&dict) as *const c_void) }
}

/// Open the Accessibility pane of System Settings.
pub fn open_settings() {
    let _ = std::process::Command::new("open").arg(SETTINGS_URL).spawn();
}

/// Open the Privacy & Security pane of System Settings.
pub fn open_privacy_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security")
        .spawn();
}

/// Has the user told macOS to never let MacClip read other apps' clipboard
/// content (the "Paste from Other Apps" privacy setting, macOS 15.4+)?
pub fn pasteboard_denied() -> bool {
    use objc2_app_kit::{NSPasteboard, NSPasteboardAccessBehavior};
    use objc2_foundation::{NSOperatingSystemVersion, NSProcessInfo};
    let supported =
        NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(NSOperatingSystemVersion {
            majorVersion: 15,
            minorVersion: 4,
            patchVersion: 0,
        });
    supported
        && NSPasteboard::generalPasteboard().accessBehavior()
            == NSPasteboardAccessBehavior::AlwaysDeny
}
