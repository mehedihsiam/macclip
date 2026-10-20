//! "Move to Applications?" prompt shown when the app is launched from a
//! disk image or the Downloads folder (login item and Accessibility both
//! depend on a stable install location).

use macclip::install;
use objc2::rc::Retained;
use objc2_app_kit::{NSAlert, NSApplication, NSRunningApplication};
use objc2_foundation::{MainThreadMarker, NSString};
use std::path::PathBuf;
use std::time::Duration;

const BUNDLE_ID: &str = "com.macclip.app";

/// Returns `true` when the app was copied and relaunched from its new
/// location, so the current process must exit.
pub fn offer_move_to_applications(mtm: MainThreadMarker) -> bool {
    if std::env::var_os("MACCLIP_SELFTEST").is_some() {
        return false;
    }
    let Some(bundle) = std::env::current_exe()
        .ok()
        .and_then(|e| install::bundle_of(&e))
    else {
        return false; // not running from a .app (cargo run)
    };
    let home = dirs::home_dir().unwrap_or_default();
    if install::is_in_applications(&bundle, &home) {
        return false;
    }

    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str("Move MacClip to Applications?"));
    alert.setInformativeText(&NSString::from_str(
        "MacClip works best when installed in your Applications folder, so it can start at login and keep its permissions. \
         MacClip will copy itself there and reopen.",
    ));
    alert.addButtonWithTitle(&NSString::from_str("Move to Applications"));
    alert.addButtonWithTitle(&NSString::from_str("Not now"));
    #[allow(deprecated)]
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    if alert.runModal() != 1000 {
        return false;
    }

    quit_other_instances();
    let targets = [PathBuf::from("/Applications"), home.join("Applications")];
    for dir in targets {
        match install::copy_bundle(&bundle, &dir) {
            Ok(dest) => {
                log::info!("installed to {}", dest.display());
                let _ = std::process::Command::new("/usr/bin/open")
                    .arg(&dest)
                    .spawn();
                return true;
            }
            Err(e) => log::warn!("could not install into {}: {e}", dir.display()),
        }
    }
    let fail = NSAlert::new(mtm);
    fail.setMessageText(&NSString::from_str("Could not install MacClip"));
    fail.setInformativeText(&NSString::from_str(
        "Please drag MacClip into your Applications folder manually. MacClip will keep running from its current location for now.",
    ));
    fail.runModal();
    false
}

/// Ask any other running MacClip (e.g. an older installed copy) to quit so
/// it can be replaced.
fn quit_other_instances() {
    let me = std::process::id() as i32;
    let apps: Retained<_> = NSRunningApplication::runningApplicationsWithBundleIdentifier(
        &NSString::from_str(BUNDLE_ID),
    );
    let mut any = false;
    for a in apps.iter() {
        if a.processIdentifier() != me {
            a.terminate();
            any = true;
        }
    }
    if any {
        std::thread::sleep(Duration::from_millis(1200));
    }
}
