//! Menu-bar icon, its right-click menu, and the controller object that
//! receives target/action messages and timer ticks.

use crate::autostart::LoginState;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSControlStateValueOn, NSEventMask, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSRect, NSString, NSTimer};

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "MacClipController"]
    pub struct Controller;

    impl Controller {
        #[unsafe(method(statusClicked:))]
        fn status_clicked(&self, _sender: Option<&AnyObject>) {
            crate::app::on_status_click();
        }

        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::app::on_tick();
        }

        #[unsafe(method(firstRun:))]
        fn first_run(&self, _timer: Option<&AnyObject>) {
            crate::app::on_first_run();
        }

        #[unsafe(method(permPoll:))]
        fn perm_poll(&self, _timer: Option<&AnyObject>) {
            crate::app::on_perm_poll();
        }

        #[unsafe(method(selfTest:))]
        fn self_test(&self, _timer: Option<&AnyObject>) {
            crate::app::on_selftest();
        }

        #[unsafe(method(togglePause:))]
        fn toggle_pause(&self, _sender: Option<&AnyObject>) {
            crate::app::on_toggle_pause();
        }

        #[unsafe(method(clearHistory:))]
        fn clear_history(&self, _sender: Option<&AnyObject>) {
            crate::app::on_clear();
        }

        #[unsafe(method(toggleLogin:))]
        fn toggle_login(&self, _sender: Option<&AnyObject>) {
            crate::app::on_toggle_login();
        }

        #[unsafe(method(accessibility:))]
        fn accessibility(&self, _sender: Option<&AnyObject>) {
            crate::app::on_accessibility();
        }

        #[unsafe(method(quitApp:))]
        fn quit_app(&self, _sender: Option<&AnyObject>) {
            crate::app::on_quit();
        }
    }
);

impl Controller {
    pub fn new(mtm: MainThreadMarker) -> Retained<Controller> {
        let this = mtm.alloc::<Controller>().set_ivars(());
        // SAFETY: NSObject's init.
        unsafe { msg_send![super(this), init] }
    }
}

/// Snapshot of state shown in the right-click menu.
pub struct MenuState {
    pub paused: bool,
    pub login: LoginState,
    pub trusted: bool,
    pub hotkey_ok: bool,
}

pub struct StatusUi {
    item: Retained<NSStatusItem>,
    pub controller: Retained<Controller>,
}

impl StatusUi {
    pub fn new(mtm: MainThreadMarker) -> StatusUi {
        let controller = Controller::new(mtm);
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        if let Some(button) = item.button(mtm) {
            let symbol = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                &NSString::from_str("doc.on.clipboard"),
                Some(&NSString::from_str("MacClip")),
            );
            match symbol {
                Some(img) => {
                    img.setTemplate(true);
                    button.setImage(Some(&img));
                }
                None => button.setTitle(&NSString::from_str("MC")),
            }
            unsafe {
                button.setTarget(Some(&controller));
                button.setAction(Some(sel!(statusClicked:)));
            }
            button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp);
        }
        StatusUi { item, controller }
    }

    /// Screen-space frame of the status icon, for anchoring the panel.
    pub fn icon_frame(&self, mtm: MainThreadMarker) -> Option<NSRect> {
        let w = self.item.button(mtm)?.window()?;
        Some(w.frame())
    }

    /// Pop up the context menu under the icon.
    pub fn show_menu(&self, mtm: MainThreadMarker, s: &MenuState) {
        let menu = self.build_menu(mtm, s);
        self.item.setMenu(Some(&menu));
        if let Some(button) = self.item.button(mtm) {
            unsafe { button.performClick(None) };
        }
        self.item.setMenu(None);
    }

    pub fn build_menu(&self, mtm: MainThreadMarker, s: &MenuState) -> Retained<NSMenu> {
        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        let c = &self.controller;

        let add = |title: &str,
                   action: Option<objc2::runtime::Sel>,
                   key: &str,
                   enabled: bool|
         -> Retained<NSMenuItem> {
            let it = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    mtm.alloc::<NSMenuItem>(),
                    &NSString::from_str(title),
                    action,
                    &NSString::from_str(key),
                )
            };
            unsafe { it.setTarget(Some(c)) };
            it.setEnabled(enabled);
            menu.addItem(&it);
            it
        };

        add(
            if s.paused {
                "Resume capture"
            } else {
                "Pause capture"
            },
            Some(sel!(togglePause:)),
            "",
            true,
        );
        add("Clear history", Some(sel!(clearHistory:)), "", true);
        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let login = add("Launch at login", Some(sel!(toggleLogin:)), "", true);
        if s.login == LoginState::Enabled {
            login.setState(NSControlStateValueOn);
        }
        if s.login == LoginState::RequiresApproval {
            login.setTitle(&NSString::from_str("Launch at login (needs approval…)"));
        }

        if s.trusted {
            add("Accessibility: granted ✓", None, "", false);
        } else {
            add(
                "Accessibility: not granted. Enable…",
                Some(sel!(accessibility:)),
                "",
                true,
            );
        }
        if !s.hotkey_ok {
            add("Shortcut ⌥V unavailable (in use)", None, "", false);
        }
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        add("Quit MacClip", Some(sel!(quitApp:)), "q", true);
        menu
    }

    /// Start the clipboard poll timer targeting the controller.
    pub fn start_timer(&self, seconds: f64) -> Retained<NSTimer> {
        unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                seconds,
                &self.controller,
                sel!(tick:),
                None,
                true,
            )
        }
    }

    pub fn schedule_first_run(&self, seconds: f64) {
        unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                seconds,
                &self.controller,
                sel!(firstRun:),
                None,
                false,
            );
        }
    }

    /// Developer aid: run the self-test once (see `App::selftest`).
    pub fn schedule_selftest(&self, seconds: f64) {
        unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                seconds,
                &self.controller,
                sel!(selfTest:),
                None,
                false,
            );
        }
    }

    /// Repeating timer used while waiting for the user to grant a permission.
    pub fn start_perm_poll(&self, seconds: f64) -> Retained<NSTimer> {
        unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                seconds,
                &self.controller,
                sel!(permPoll:),
                None,
                true,
            )
        }
    }
}
