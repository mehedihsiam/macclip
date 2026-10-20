//! MacClip — Clipboard history for macOS.

mod app;
mod autostart;
mod installer;
mod paste;
mod pasteboard;
mod permissions;
mod ui;

use app::App;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use macclip::config::{self, Config};
use macclip::store::Store;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::MainThreadMarker;
use std::cell::{Cell, RefCell};
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::rc::Rc;

fn init_logging() {
    let dir = config::log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("macclip.log");
    // Keep the log small: start fresh once it grows past 1 MB.
    if std::fs::metadata(&path)
        .map(|m| m.len() > 1_000_000)
        .unwrap_or(false)
    {
        let _ = std::fs::remove_file(&path);
    }
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    if let Ok(f) = OpenOptions::new().create(true).append(true).open(&path) {
        macclip::store::restrict_file(&path);
        builder.target(env_logger::Target::Pipe(Box::new(f)));
    }
    builder.format_timestamp_secs().init();
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
}

/// Hold an exclusive lock for the process lifetime; `None` means another
/// instance already runs.
fn single_instance(dir: &std::path::Path) -> Option<std::fs::File> {
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("macclip.lock"))
        .ok()?;
    let rc = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    (rc == 0).then_some(f)
}

fn main() {
    init_logging();
    let mtm = MainThreadMarker::new().expect("main thread");
    if installer::offer_move_to_applications(mtm) {
        return;
    }
    let data_dir = config::data_dir();
    let store = match Store::open(&data_dir) {
        Ok(s) => s,
        Err(e) => {
            log::error!("cannot open store: {e:#}");
            eprintln!("MacClip: cannot open history store: {e:#}");
            std::process::exit(1);
        }
    };
    let Some(_lock) = single_instance(&data_dir) else {
        log::info!("another MacClip instance is running; exiting");
        return;
    };
    let cfg = Config::load_or_create(&data_dir.join("config.json"));

    let ns_app = NSApplication::sharedApplication(mtm);
    ns_app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let status = ui::status_item::StatusUi::new(mtm);
    let poll_secs = cfg.poll_interval_ms as f64 / 1000.0;
    let app = Rc::new(App {
        mtm,
        cfg,
        store: RefCell::new(store),
        tracker: RefCell::new(macclip::monitor::ChangeTracker::new()),
        hotkey_ok: Cell::new(false),
        paste_help_shown: Cell::new(false),
        perm_timer: RefCell::new(None),
        perm_attempts: Cell::new(0),
        panel: ui::panel::Panel::new(mtm),
        status,
        hotkey: RefCell::new(None),
        timers: RefCell::new(Vec::new()),
        prev_app: RefCell::new(None),
        model: RefCell::new(Default::default()),
    });
    app::install(app.clone());

    // Option+V
    match GlobalHotKeyManager::new().and_then(|m| {
        m.register(HotKey::new(Some(Modifiers::ALT), Code::KeyV))?;
        Ok(m)
    }) {
        Ok(m) => {
            app.hotkey_ok.set(true);
            *app.hotkey.borrow_mut() = Some(m);
            GlobalHotKeyEvent::set_event_handler(Some(|e: GlobalHotKeyEvent| {
                if e.state() == HotKeyState::Pressed {
                    dispatch2::DispatchQueue::main().exec_async(app::on_hotkey);
                }
            }));
        }
        Err(e) => log::warn!("Option+V unavailable: {e}"),
    }

    let t = app.status.start_timer(poll_secs);
    app.timers.borrow_mut().push(t);
    if std::env::var_os("MACCLIP_SELFTEST").is_some() {
        app.status.schedule_selftest(2.0);
    } else {
        app.status.schedule_first_run(0.8);
    }

    log::info!("MacClip started");
    ns_app.run();
}
