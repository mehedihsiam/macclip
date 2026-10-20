//! Application state and the glue between clipboard, store, panel and menu.

use crate::autostart::{self, LoginState};
use crate::ui::panel::{MAX_VISIBLE, Panel, View, WIDTH};
use crate::ui::status_item::{MenuState, StatusUi};
use crate::{paste, pasteboard, permissions};
use global_hotkey::GlobalHotKeyManager;
use macclip::config::{self, Config};
use macclip::monitor::{ChangeTracker, Observation};
use macclip::privacy;
use macclip::store::{Entry, Store};
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAlert, NSApplication, NSApplicationActivationOptions, NSEvent, NSEventModifierFlags,
    NSEventType, NSRunningApplication, NSScreen, NSWorkspace,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSTimer};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[derive(Default)]
pub struct Model {
    pub query: String,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub top: usize,
}

pub struct App {
    pub mtm: MainThreadMarker,
    pub cfg: Config,
    pub store: RefCell<Store>,
    pub tracker: RefCell<ChangeTracker>,
    pub hotkey_ok: Cell<bool>,
    pub panel: Panel,
    pub status: StatusUi,
    pub hotkey: RefCell<Option<GlobalHotKeyManager>>,
    pub timers: RefCell<Vec<Retained<NSTimer>>>,
    pub prev_app: RefCell<Option<Retained<NSRunningApplication>>>,
    pub model: RefCell<Model>,
}

thread_local! {
    static APP: RefCell<Option<Rc<App>>> = const { RefCell::new(None) };
}

pub fn install(app: Rc<App>) {
    APP.with(|a| *a.borrow_mut() = Some(app));
}

fn current() -> Option<Rc<App>> {
    APP.with(|a| a.borrow().clone())
}

fn with_app(f: impl FnOnce(&Rc<App>)) {
    if let Some(a) = current() {
        f(&a);
    }
}

// ---- entry points called from Objective-C callbacks ------------------------

pub fn on_key(e: &NSEvent) {
    with_app(|a| a.handle_key(e));
}
pub fn on_escape() {
    with_app(|a| a.close_panel());
}
pub fn on_resign_key() {
    with_app(|a| a.close_panel());
}
pub fn on_click(row: usize) {
    with_app(|a| a.click_row(row));
}
pub fn on_scroll(dy: f64) {
    with_app(|a| a.scroll(dy));
}
pub fn on_tick() {
    with_app(|a| a.poll());
}
pub fn on_status_click() {
    with_app(|a| a.status_clicked());
}
pub fn on_hotkey() {
    with_app(|a| a.toggle_panel(Anchor::Mouse));
}
pub fn on_selftest() {
    if let Some(dir) = std::env::var_os("MACCLIP_SELFTEST") {
        with_app(|a| a.selftest(std::path::Path::new(&dir)));
    }
}
pub fn on_first_run() {
    with_app(|a| a.first_run());
}
pub fn on_toggle_pause() {
    with_app(|a| {
        let paused = !a.tracker.borrow().paused();
        a.tracker.borrow_mut().set_paused(paused);
        log::info!("capture {}", if paused { "paused" } else { "resumed" });
    });
}
pub fn on_clear() {
    with_app(|a| {
        if let Err(e) = a.store.borrow_mut().clear() {
            log::error!("clear failed: {e:#}");
        }
        a.reload();
    });
}
pub fn on_toggle_login() {
    with_app(|a| a.toggle_login());
}
pub fn on_accessibility() {
    permissions::prompt();
    permissions::open_settings();
}
pub fn on_quit() {
    with_app(|a| NSApplication::sharedApplication(a.mtm).terminate(None));
}

#[derive(Clone, Copy)]
pub enum Anchor {
    Icon,
    Mouse,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn frontmost() -> (Option<String>, Option<String>) {
    match NSWorkspace::sharedWorkspace().frontmostApplication() {
        Some(app) => (
            app.bundleIdentifier().map(|s| s.to_string()),
            app.localizedName().map(|s| s.to_string()),
        ),
        None => (None, None),
    }
}

impl App {
    // ---- clipboard monitoring ---------------------------------------------

    /// Called every poll interval; reads the pasteboard only when it changed.
    pub fn poll(&self) {
        let obs = self
            .tracker
            .borrow_mut()
            .observe(pasteboard::change_count());
        if obs != Observation::Capture {
            return;
        }
        let snap = pasteboard::read_snapshot();
        let (bundle, name) = frontmost();
        match privacy::build_item(snap, bundle.as_deref(), name.as_deref(), &self.cfg) {
            Ok(item) => {
                match self.store.borrow_mut().add(&item) {
                    Ok(_) => log::debug!("captured {:?} item", item.kind()),
                    Err(e) => log::error!("store add failed: {e:#}"),
                }
                if self.panel.is_visible() {
                    self.refilter();
                }
            }
            Err(skip) => log::debug!("skipped clipboard item: {skip:?}"),
        }
    }

    // ---- panel ------------------------------------------------------------

    fn status_clicked(&self) {
        let ev = NSApplication::sharedApplication(self.mtm).currentEvent();
        let right = ev.as_ref().is_some_and(|e| {
            e.r#type() == NSEventType::RightMouseUp
                || e.modifierFlags().contains(NSEventModifierFlags::Control)
        });
        if right {
            self.close_panel();
            self.status.show_menu(
                self.mtm,
                &MenuState {
                    paused: self.tracker.borrow().paused(),
                    login: autostart::state(),
                    trusted: permissions::is_trusted(),
                    hotkey_ok: self.hotkey_ok.get(),
                },
            );
        } else {
            self.toggle_panel(Anchor::Icon);
        }
    }

    pub fn toggle_panel(&self, anchor: Anchor) {
        if self.panel.is_visible() {
            self.close_panel();
        } else {
            self.open_panel(anchor);
        }
    }

    fn open_panel(&self, anchor: Anchor) {
        *self.prev_app.borrow_mut() = NSWorkspace::sharedWorkspace().frontmostApplication();
        {
            let mut m = self.model.borrow_mut();
            *m = Model::default();
            m.entries = self.store.borrow().list().unwrap_or_default();
        }
        self.render();
        let size = self.panel.size();
        let top_left = self.anchor_point(anchor, size.height);
        self.panel.show(top_left);
        // Re-render once visible so the layout matches the final frame.
        self.render();
    }

    fn anchor_point(&self, anchor: Anchor, height: f64) -> NSPoint {
        let mouse = NSEvent::mouseLocation();
        let screens = NSScreen::screens(self.mtm);
        let contains = |r: NSRect, p: NSPoint| {
            p.x >= r.origin.x
                && p.x <= r.origin.x + r.size.width
                && p.y >= r.origin.y
                && p.y <= r.origin.y + r.size.height
        };
        match anchor {
            Anchor::Icon => {
                if let Some(icon) = self.status.icon_frame(self.mtm) {
                    let centre = NSPoint::new(icon.origin.x + icon.size.width / 2.0, icon.origin.y);
                    let screen = screens.iter().find(|s| contains(s.frame(), centre));
                    let vf = screen.map(|s| s.visibleFrame());
                    let mut x = centre.x - WIDTH / 2.0;
                    if let Some(vf) = vf {
                        x = x.clamp(vf.origin.x + 8.0, vf.origin.x + vf.size.width - WIDTH - 8.0);
                    }
                    return NSPoint::new(x, icon.origin.y - 4.0);
                }
                self.mouse_anchor(mouse, height)
            }
            Anchor::Mouse => self.mouse_anchor(mouse, height),
        }
    }

    fn mouse_anchor(&self, mouse: NSPoint, height: f64) -> NSPoint {
        let screens = NSScreen::screens(self.mtm);
        let hit = screens.iter().find(|s| {
            let r = s.frame();
            mouse.x >= r.origin.x
                && mouse.x <= r.origin.x + r.size.width
                && mouse.y >= r.origin.y
                && mouse.y <= r.origin.y + r.size.height
        });
        let vf = hit
            .map(|s| s.visibleFrame())
            .or_else(|| NSScreen::mainScreen(self.mtm).map(|s| s.visibleFrame()))
            .unwrap_or(NSRect::new(
                NSPoint::new(0.0, 0.0),
                objc2_foundation::NSSize::new(1440.0, 900.0),
            ));
        let x = vf.origin.x + (vf.size.width - WIDTH) / 2.0;
        let bottom = vf.origin.y + (vf.size.height - height) / 2.0 + vf.size.height * 0.08;
        NSPoint::new(x, bottom + height)
    }

    pub fn close_panel(&self) {
        if self.panel.is_visible() {
            self.panel.hide();
        }
    }

    fn render(&self) {
        let m = self.model.borrow();
        self.panel.render(
            self.mtm,
            &View {
                entries: &m.entries,
                query: &m.query,
                selected: m.selected,
                top: m.top,
                trusted: permissions::is_trusted(),
                now_ms: now_ms(),
            },
        );
    }

    /// Reload entries from the store (keeping the query) and redraw.
    fn refilter(&self) {
        {
            let mut m = self.model.borrow_mut();
            m.entries = self.store.borrow().search(&m.query).unwrap_or_default();
            m.selected = 0;
            m.top = 0;
        }
        self.render();
    }

    fn reload(&self) {
        if self.panel.is_visible() {
            self.refilter();
        }
    }

    fn move_selection(&self, delta: isize) {
        {
            let mut m = self.model.borrow_mut();
            if m.entries.is_empty() {
                return;
            }
            let last = m.entries.len() as isize - 1;
            m.selected = (m.selected as isize + delta).clamp(0, last) as usize;
            if m.selected < m.top {
                m.top = m.selected;
            } else if m.selected >= m.top + MAX_VISIBLE {
                m.top = m.selected + 1 - MAX_VISIBLE;
            }
        }
        self.render();
    }

    fn scroll(&self, dy: f64) {
        {
            let mut m = self.model.borrow_mut();
            let max_top = m.entries.len().saturating_sub(MAX_VISIBLE);
            if dy > 0.5 {
                m.top = m.top.saturating_sub(1);
            } else if dy < -0.5 {
                m.top = (m.top + 1).min(max_top);
            } else {
                return;
            }
        }
        self.render();
    }

    fn click_row(&self, row: usize) {
        let idx = self.model.borrow().top + row;
        self.choose(idx);
    }

    fn handle_key(&self, ev: &NSEvent) {
        let blockers = NSEventModifierFlags::Command
            | NSEventModifierFlags::Control
            | NSEventModifierFlags::Option;
        let blocked = ev.modifierFlags().intersects(blockers);
        let chars = ev.characters().map(|c| c.to_string());
        self.key(ev.keyCode(), chars.as_deref(), blocked);
    }

    /// Key handling shared by real events and the self-test.
    /// `blocked` is true when Command/Control/Option is held.
    pub fn key(&self, code: u16, chars: Option<&str>, blocked: bool) {
        match code {
            53 => self.close_panel(),       // Escape
            125 => self.move_selection(1),  // Down
            126 => self.move_selection(-1), // Up
            36 | 76 => {
                let sel = self.model.borrow().selected;
                self.choose(sel); // Return / Enter
            }
            51 => {
                let changed = self.model.borrow_mut().query.pop().is_some(); // Delete
                if changed {
                    self.refilter();
                }
            }
            _ => {
                if blocked {
                    return;
                }
                let Some(chars) = chars else { return };
                for ch in chars.chars() {
                    let empty = self.model.borrow().query.is_empty();
                    if empty && ('1'..='9').contains(&ch) {
                        self.choose(ch as usize - '1' as usize);
                        return;
                    }
                    if !ch.is_control() {
                        self.model.borrow_mut().query.push(ch);
                        self.refilter();
                    }
                }
            }
        }
    }

    // ---- paste ------------------------------------------------------------

    /// Paste the entry at `idx` of the currently displayed list.
    fn choose(&self, idx: usize) {
        let id = match self.model.borrow().entries.get(idx) {
            Some(e) => e.id,
            None => return,
        };
        let payload = match self.store.borrow().payload(id) {
            Ok(Some(p)) => p,
            _ => {
                log::warn!("entry {id} vanished");
                return;
            }
        };
        let _ = self.store.borrow_mut().bump(id);
        let cc = pasteboard::write_payload(&payload);
        self.tracker.borrow_mut().mark_own_write(cc);
        self.close_panel();

        // Give focus back explicitly if it drifted.
        if let Some(prev) = self.prev_app.borrow().as_ref() {
            let front = NSWorkspace::sharedWorkspace().frontmostApplication();
            if front.as_ref().map(|f| f.processIdentifier()) != Some(prev.processIdentifier()) {
                #[allow(deprecated)]
                prev.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
            }
        }

        if permissions::is_trusted() {
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(90));
                paste::send_cmd_v();
            });
        } else {
            paste::notify(
                "Copied to clipboard",
                "Press ⌘V to paste. Enable MacClip in Privacy & Security → Accessibility for auto-paste.",
            );
        }
    }

    // ---- first run, login item -------------------------------------------

    fn first_run(&self) {
        let flag = config::data_dir().join("onboarded");
        if flag.exists() {
            return;
        }
        match autostart::set_enabled(true) {
            Ok(LoginState::RequiresApproval) => {
                paste::notify(
                    "MacClip",
                    "Approve MacClip in System Settings → General → Login Items to start at login.",
                );
                autostart::open_login_items_settings();
            }
            Ok(s) => log::info!("login item: {s:?}"),
            Err(e) => log::warn!("could not register login item (run from MacClip.app): {e}"),
        }
        if !permissions::is_trusted() {
            let alert = NSAlert::new(self.mtm);
            alert.setMessageText(&objc2_foundation::NSString::from_str(
                "MacClip needs Accessibility access",
            ));
            alert.setInformativeText(&objc2_foundation::NSString::from_str(
                "To paste a history item straight into the field you are typing in, MacClip must send a ⌘V key press. \
                 macOS requires your permission for that.\n\nMacClip never reads your keystrokes and never sends data off your Mac. \
                 Without this permission, selecting an item still copies it so you can press ⌘V yourself.",
            ));
            alert.addButtonWithTitle(&objc2_foundation::NSString::from_str("Continue"));
            alert.addButtonWithTitle(&objc2_foundation::NSString::from_str("Not now"));
            #[allow(deprecated)]
            NSApplication::sharedApplication(self.mtm).activateIgnoringOtherApps(true);
            if alert.runModal() == 1000 {
                permissions::prompt();
            }
        }
        let _ = std::fs::write(&flag, b"1");
    }

    fn toggle_login(&self) {
        let enable = autostart::state() != LoginState::Enabled;
        match autostart::set_enabled(enable) {
            Ok(LoginState::RequiresApproval) => {
                paste::notify(
                    "MacClip",
                    "Approve MacClip in System Settings → General → Login Items.",
                );
                autostart::open_login_items_settings();
            }
            Ok(_) => {}
            Err(e) => paste::notify("MacClip: login item failed", &e),
        }
    }
}

// ---- self-test (developer aid) ---------------------------------------------

impl App {
    /// Drives the real panel logic and writes evidence into `dir`.
    /// Enabled by launching with `MACCLIP_SELFTEST=<dir>`; never runs otherwise.
    pub fn selftest(&self, dir: &std::path::Path) {
        use std::fmt::Write as _;
        let mut out = String::new();
        let snap = |name: &str, a: &App| {
            if let Some(png) = a.panel.snapshot_png() {
                let _ = std::fs::write(dir.join(name), png);
            }
        };
        let front_before = frontmost().0;
        self.open_panel(Anchor::Mouse);
        let _ = writeln!(out, "panel visible: {}", self.panel.is_visible());
        let _ = writeln!(out, "panel is key window: {}", self.panel.is_key());
        let _ = writeln!(
            out,
            "frontmost before: {front_before:?}, after open: {:?}",
            frontmost().0
        );
        let _ = writeln!(out, "entries listed: {}", self.model.borrow().entries.len());
        snap("panel-all.png", self);

        self.key(125, None, false);
        self.key(125, None, false);
        let _ = writeln!(
            out,
            "selected after 2x down: {}",
            self.model.borrow().selected
        );
        snap("panel-selected.png", self);

        for c in "item 1".chars() {
            self.key(0, Some(&c.to_string()), false);
        }
        let _ = writeln!(
            out,
            "query 'item 1' matches: {}",
            self.model.borrow().entries.len()
        );
        snap("panel-search.png", self);
        for _ in 0..6 {
            self.key(51, None, false);
        }
        let _ = writeln!(
            out,
            "after clearing query: {}",
            self.model.borrow().entries.len()
        );

        // option held: must not type
        self.key(0, Some("z"), true);
        let _ = writeln!(out, "query after Option+z: {:?}", self.model.borrow().query);

        // quick-pick "2" => second entry
        let second = self
            .model
            .borrow()
            .entries
            .get(1)
            .map(|e| e.preview.clone());
        self.key(0, Some("2"), false);
        let pb = pasteboard::read_snapshot();
        let _ = writeln!(
            out,
            "quick-pick 2 expected {second:?}, pasteboard text {:?}",
            pb.text
        );
        let _ = writeln!(
            out,
            "panel visible after choose: {}",
            self.panel.is_visible()
        );
        let before: Vec<_> = self
            .store
            .borrow()
            .list()
            .unwrap_or_default()
            .iter()
            .map(|e| e.id)
            .collect();
        self.poll();
        let after: Vec<_> = self
            .store
            .borrow()
            .list()
            .unwrap_or_default()
            .iter()
            .map(|e| e.id)
            .collect();
        let _ = writeln!(out, "own write ignored by monitor: {}", before == after);
        let top = self.store.borrow().list().unwrap_or_default();
        let _ = writeln!(
            out,
            "top entry after choose: {:?}",
            top.first().map(|e| e.preview.clone())
        );
        let _ = writeln!(out, "accessibility trusted: {}", permissions::is_trusted());

        let _ = writeln!(
            out,
            "status icon frame: {:?}",
            self.status.icon_frame(self.mtm).map(|r| (
                r.origin.x,
                r.origin.y,
                r.size.width,
                r.size.height
            ))
        );
        let menu = self.status.build_menu(
            self.mtm,
            &MenuState {
                paused: self.tracker.borrow().paused(),
                login: autostart::state(),
                trusted: permissions::is_trusted(),
                hotkey_ok: self.hotkey_ok.get(),
            },
        );
        let titles: Vec<String> = menu
            .itemArray()
            .iter()
            .map(|i| {
                if i.isSeparatorItem() {
                    "---".to_string()
                } else {
                    format!("{} [enabled={}]", i.title(), i.isEnabled())
                }
            })
            .collect();
        let _ = writeln!(out, "menu: {titles:?}");
        let _ = writeln!(out, "hotkey registered: {}", self.hotkey_ok.get());

        // escape closes
        self.open_panel(Anchor::Icon);
        let _ = writeln!(
            out,
            "icon-anchored open visible: {}",
            self.panel.is_visible()
        );
        snap("panel-icon.png", self);
        self.key(53, None, false);
        let _ = writeln!(out, "visible after Escape: {}", self.panel.is_visible());

        let _ = std::fs::write(dir.join("result.txt"), out);
    }
}
