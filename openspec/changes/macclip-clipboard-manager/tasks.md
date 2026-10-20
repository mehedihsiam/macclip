# Tasks

## 1. Toolchain and scaffold

- [x] 1.1 Install Rust via rustup and add `aarch64-apple-darwin` and `x86_64-apple-darwin` targets; verify `cargo --version` and `rustup target list --installed`
- [x] 1.2 Create `macclip` crate with release profile (lto, strip, panic=abort), dependencies from design.md, `.gitignore`, `git init`; verify `cargo check` passes
- [x] 1.3 Add module skeleton (`monitor`, `privacy`, `store`, `paste`, `permissions`, `autostart`, `config`, `ui/`) and `config.rs` loading `config.json` with defaults (exclude list, poll interval, size caps); verify unit test for default and override loading

## 2. History store

- [x] 2.1 Implement `store.rs` (SQLite schema, hash dedupe insert-or-bump, trim to 20 in one transaction, list newest first) with 0700/0600 permissions; verify tests for overflow, dedupe, ordering and permissions
- [x] 2.2 Implement image + thumbnail file handling and orphan deletion on eviction/clear; verify tests that files are removed
- [x] 2.3 Implement persistence across reopen; verify test that reopening the DB returns identical ordered entries

## 3. Clipboard monitoring and privacy

- [x] 3.1 Define pasteboard trait and `privacy.rs` (concealed/transient/auto-generated markers, excluded frontmost app, size caps); verify unit tests with fake pasteboard
- [x] 3.2 Implement NSPasteboard-backed reader and 300 ms changeCount timer capturing text, RTF, HTML, PNG/TIFF as one entry; verify with `pbcopy`/image copy that entries appear within 1 s
- [x] 3.3 Implement self-write guard and pause/resume; verify tests that own writes and paused state create no entries

## 4. Menu bar and hotkey

- [x] 4.1 Bootstrap NSApplication as Accessory app with single-instance guard and panic/log setup; verify no Dock icon and a second launch exits
- [x] 4.2 Implement `ui/status_item.rs` icon, left-click toggle and right-click menu (Pause/Resume, Clear, Launch at login, Accessibility status, Quit); verify icon and menu items manually
- [ ] 4.3 Register Option+V via `global-hotkey` and surface registration failure in the menu; verify shortcut toggles the panel from another app

## 5. History panel

- [x] 5.1 Implement non-activating NSPanel with vibrancy, positioning under icon or on the mouse's screen; verify previous app remains frontmost when opened
- [x] 5.2 Render rows (preview, thumbnail, source app, relative time) and empty state; verify with mixed text/image history
- [ ] 5.3 Implement Up/Down/Return/Escape/1–9, search filtering and click-outside/Space-change dismissal; verify each key path manually

## 6. Paste and permissions

- [ ] 6.1 Implement `permissions.rs` (AXIsProcessTrusted check, first-run explainer, system prompt, Settings deep link, polling after prompt); verify status updates after granting without restart
- [ ] 6.2 Implement `paste.rs` (restore all representations, bump entry, focus guard, CGEvent ⌘V, copy-only fallback with notification); verify paste into TextEdit with and without permission
- [ ] 6.3 Verify rich-text and image entries paste with formatting/content preserved into TextEdit/Notes/Preview

## 7. Autostart

- [ ] 7.1 Implement `autostart.rs` using SMAppService (register, toggle, requires-approval guidance); verify login item appears in System Settings and app launches after re-login

## 8. Packaging and docs

- [x] 8.1 Create `packaging/Info.plist` and icon generation script; verify `plutil -p` shows MacClip, com.macclip.app, LSUIElement, 13.0
- [x] 8.2 Create `scripts/build_app.sh` (two-target build, lipo, bundle, ad-hoc codesign, `--install`); verify `lipo -info`, `codesign -dv --verbose=2` and `codesign --verify`
- [x] 8.3 Write README (build, install, permissions, re-grant note, privacy) and run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`; verify all pass
- [ ] 8.4 Run full end-to-end checklist from the plan (25 copies → 20 kept, image copy, Option+V, icon click, concealed skip, permission revoke/grant, relaunch persistence); verify each passes
