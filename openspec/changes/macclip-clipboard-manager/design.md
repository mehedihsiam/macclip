# Design

## Context

Greenfield Rust project; no code exists. The Rust toolchain is not yet installed (Xcode CLT is, macOS 26.4 host). The app must be a long-running agent process with AppKit UI, so the main thread owns the Cocoa run loop. See proposal.md for motivation and specs/ for behavior.

## Goals / Non-Goals

**Goals:**
- Native, tiny, low-idle-CPU background app, 100% Rust.
- Testable core (store, privacy filter, dedupe) separated from AppKit glue behind traits.
- Only one permission needed (Accessibility), requested with context.

**Non-Goals:**
- Cross-platform support, a webview UI, cloud sync.
- Configurable hotkey UI, pinned/favourite items, Finder file entries (v1).
- Developer ID signing / notarization.

## Decisions

1. **Native AppKit via `objc2` crates (not Tauri/egui).** Smallest footprint and exact control of `NSStatusItem`/`NSPanel` focus behavior. Alternative: Tauri (heavier, webview), egui (poor menu-bar/non-activating panel support).
2. **Poll `NSPasteboard.changeCount` every 300 ms** on a main-run-loop timer. macOS has no clipboard-change notification; polling an integer is negligible CPU. Alternative: none viable.
3. **SQLite (`rusqlite` bundled) for metadata + PNG files on disk for images.** Transactional insert-or-bump + trim keeps the 20-item invariant crash-safe; blobs stay out of the DB. Alternative: JSON file (simple but non-atomic, weaker de-dupe queries).
4. **Dedupe by SHA-256 of canonical content** (text bytes, or image pixel/PNG bytes). Hash is UNIQUE; re-copy updates `created_at`.
5. **Non-activating `NSPanel` (`NonactivatingPanel`, `canBecomeKey`) shared by hotkey and icon.** The previous app and its first responder keep focus, so ⌘V lands correctly; the frontmost app is recorded at open and re-activated if focus drifted. Alternative: NSMenu/NSPopover (activates app or poor thumbnail/search support).
6. **Hotkey via `global-hotkey` (Carbon `RegisterEventHotKey`).** Requires no Accessibility/Input Monitoring permission, preserving "one permission" goal. Alternative: CGEventTap (needs Input Monitoring).
7. **Paste = write pasteboard with all representations → record changeCount (self-write guard) → close panel → ~50 ms later post CGEvent ⌘V.** Accessibility checked with `AXIsProcessTrustedWithOptions`; untrusted → copy-only plus user notification.
8. **Login item via `SMAppService.mainApp`** (macOS 13+), not a hand-written LaunchAgent plist; user-visible and removable in System Settings.
9. **Privacy by default:** skip nspasteboard.org concealed/transient/auto-generated markers and an app exclude list in `config.json`; no network code; 0700/0600 permissions.
10. **Packaging via shell script:** build both Rust targets, `lipo`, assemble bundle, ad-hoc `codesign` with stable identifier `com.macclip.app`.
11. **Module layout:** `monitor`, `privacy`, `store`, `paste`, `permissions`, `autostart`, `config`, `ui/{status_item,panel}`. Pasteboard access sits behind a trait so monitor/privacy are unit-tested with fakes.

## Risks / Trade-offs

- [Ad-hoc signature changes per rebuild, so Accessibility grant may be reset] → stable bundle id, documented re-grant step; Developer ID later.
- [macOS 15.4+ pasteboard privacy alerts for programmatic reads] → read only on changeCount change, surface guidance if access is denied.
- [Option+V already used / produces "√"] → registration result surfaced in menu.
- [Secure input fields block synthetic ⌘V] → clipboard still set; notification fallback.
- [objc2 API verbosity and main-thread rules] → keep all AppKit calls on main thread via `MainThreadMarker`; isolate in `ui/`.
- [Polling latency up to 300 ms] → within the 1 s spec requirement.
- [Large images] → size caps and PNG re-encode off the main thread where practical.
