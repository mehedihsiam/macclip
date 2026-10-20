# Proposal

## Why

macOS keeps only the most recent clipboard item, so anything copied earlier is lost the moment something new is copied. MacClip is a lightweight, privacy-respecting, native clipboard history manager that lives in the menu bar and lets users re-paste any of their recent copies, including images, in one keystroke.

## What Changes

- New Rust menu-bar app named **MacClip – Clipboard history for macOS** (no Dock icon).
- Continuously records the last 20 clipboard entries: plain text, rich text (RTF/HTML) and images; duplicates move to the top instead of repeating.
- History persists on disk across restarts; sensitive (concealed/transient) copies and excluded apps are never recorded.
- One floating history panel opened by **Option+V** or by clicking the menu-bar icon; keyboard navigation, search, image thumbnails.
- Selecting an entry pastes it immediately into the previously focused field (synthetic ⌘V) after Accessibility permission onboarding; graceful copy-only fallback if not granted.
- Starts automatically at login (login item) and runs in the background.
- Packaged as an ad-hoc-signed universal (arm64 + x86_64) `MacClip.app` for macOS 13+.

## Capabilities

### New Capabilities
- `clipboard-monitoring`: Detecting clipboard changes and capturing text, rich text and image content, with privacy filtering.
- `history-store`: Bounded (20-item) persistent history with de-duplication, ordering, size limits and secure local storage.
- `history-panel`: The floating, searchable, keyboard-driven history list with image thumbnails.
- `menubar-and-hotkey`: Menu-bar status icon, its context menu, and the global Option+V shortcut.
- `paste-selection`: Writing a chosen entry back to the clipboard and pasting it into the previously focused field.
- `permissions`: Accessibility permission detection, onboarding and degraded behavior.
- `autostart`: Launch at login and single-instance background operation.
- `app-packaging`: App identity, bundle metadata and the build/sign/install process.

### Modified Capabilities

(none — greenfield project)

## Impact

- New Rust crate (`macclip`) and `scripts/`, `packaging/`, `assets/` directories; no existing code is affected.
- Dependencies: objc2 family (AppKit, Foundation, ServiceManagement), global-hotkey, core-graphics, rusqlite (bundled), image, sha2, serde/serde_json, dirs, log, anyhow.
- System integration: Accessibility permission, Login Items (SMAppService), `~/Library/Application Support/MacClip`, `~/Library/Logs/MacClip`.
- Requires installing the Rust toolchain (currently absent) with both darwin targets.
- Out of scope for v1: user-configurable hotkey UI, iCloud/cross-device sync, pinned items, Developer ID signing/notarization, file-copy (Finder) entries.
