# MacClip — Developer documentation

> End-user manual: see the [README](../README.md).


MacClip is a tiny, native clipboard history manager for macOS, written entirely in Rust. It lives in your menu bar, quietly remembers your last 20 copies (text, rich text and images), and lets you paste any of them back with one shortcut: **Option + V**.

> **Status:** v0.1 implemented and unit tested; the app builds, installs and runs. Features that need your interaction (granting Accessibility, pressing Option+V from another app, re-login) still need a manual check, see [Verification checklist](#verification-checklist). Requirements live in [`openspec/changes/macclip-clipboard-manager/`](../openspec/changes/macclip-clipboard-manager/); an HTML version of these docs is in [`docs/index.html`](index.html).

---

## Contents

1. [Features](#features)
2. [How to use it](#how-to-use-it)
3. [Permissions](#permissions)
4. [Privacy and security](#privacy-and-security)
5. [How it works](#how-it-works)
6. [Where your data lives](#where-your-data-lives)
7. [Configuration](#configuration)
8. [Building and installing](#building-and-installing)
9. [Project layout](#project-layout)
10. [Limitations and troubleshooting](#limitations-and-troubleshooting)
11. [Development workflow](#development-workflow)

---

## Features

| Feature | Details |
|---|---|
| Last 20 copies | Keeps the 20 most recent clipboard entries, newest first. The oldest is dropped when a 21st arrives. |
| Text, rich text, images | Plain text, RTF, HTML and images (PNG/TIFF) are all captured. Formatting is preserved when you paste back. |
| No duplicates | Copying something you already copied moves it to the top instead of adding another row. |
| Option + V | Opens the history from any app, on the screen where your mouse is. |
| Menu-bar icon | Sits in the top-right status area. Click it to open the same history list. |
| Instant paste | Pick an entry and it is pasted straight into the field you were typing in. |
| Compact, quiet panel | A small 320 px panel with single-line rows, faint relative times and a subtle selection tint. |
| Search | Start typing in the panel to filter entries. |
| Keyboard first | Arrow keys, Return, Escape, and number keys 1–9 for quick picks. |
| Starts at login | Registers as a login item so it is always running. No Dock icon. |
| Survives restarts | History is saved on disk and is still there after reboot. |
| Privacy aware | Ignores passwords from password managers and apps you exclude. Nothing leaves your Mac. |
| Pause / Clear | Pause recording or wipe history from the menu-bar menu. |

---

## How to use it

### Open the history

- Press **Option + V** in any app, or
- **Left-click** the MacClip icon in the menu bar (it appears next to your other menu-bar icons, top right).

The panel appears without taking focus from the app you are working in, so your cursor stays in your text field.

### Paste an entry

| Action | Result |
|---|---|
| `↑` / `↓` | Move the selection |
| `Return` | Paste the selected entry |
| `1` – `9` | Paste the 1st–9th entry immediately (when the search box is empty; once you have typed a search, digits become part of it) |
| Click a row | Paste that entry |
| Type text | Filter the list by content or source-app name (case-insensitive); `Delete` edits the search |
| `Esc` or click outside | Close without pasting |

### The menu-bar menu (right-click the icon)

- **Pause / Resume capture**: stop recording temporarily.
- **Clear history**: delete every entry and its stored image files.
- **Launch at login**: toggle automatic start.
- **Accessibility status**: shows whether auto-paste is enabled, with a shortcut to System Settings.
- **Quit**.

---

## Permissions

MacClip needs **one** permission, and works without it in a reduced mode.

| Permission | Why | If missing |
|---|---|---|
| **Accessibility** | To simulate ⌘V so the selected entry is pasted into the focused field. | The entry is still placed on your clipboard and a notification tells you to press ⌘V yourself. |

Not required: Input Monitoring, Screen Recording, Full Disk Access, or network access. The Option + V shortcut uses the system hot-key API, which needs no permission.

On first launch MacClip explains why it needs Accessibility, then shows the standard macOS prompt. Grant it in **System Settings → Privacy & Security → Accessibility**. MacClip notices the change on its own; no restart is needed.

---

## Privacy and security

- **Local only.** There is no networking code. History never leaves your Mac.
- **Sensitive copies are skipped.** Items flagged by password managers and other apps as *concealed*, *transient* or *auto-generated* (the `org.nspasteboard.*` convention) are never recorded.
- **App exclusion list.** Copies made while an excluded app is frontmost are ignored. Defaults: 1Password, Bitwarden, Keychain Access, Dashlane.
- **Locked-down storage.** The data folder is owner-only (`0700`) and files are `0600`.
- **Size limits.** Text over 1 MB and images over 25 MB are skipped.
- **Bounded exposure.** Only 20 entries ever exist; evicted and cleared entries have their image files deleted.

> Anything you copy from a source that does *not* flag itself as sensitive will be stored in plain form on disk until it is evicted or cleared. Use **Pause** when handling secrets from such sources.

---

## How it works

### Architecture overview

```
 ┌────────────┐   changeCount poll    ┌──────────────┐
 │ NSPasteboard│ ───────(300 ms)─────▶ │   monitor    │
 └────────────┘                        └──────┬───────┘
                                              │ ClipItem
                                       ┌──────▼───────┐   blocks concealed /
                                       │   privacy    │   excluded-app / oversize
                                       └──────┬───────┘
                                              │
                                       ┌──────▼───────┐    SQLite + PNG files
                                       │    store     │ ─▶ ~/Library/Application Support/MacClip
                                       └──────┬───────┘
                       Option+V ┐             │ list (≤ 20)
                       Icon click ┴─▶ ┌───────▼───────┐
                                      │  panel (UI)   │
                                      └───────┬───────┘
                                              │ selection
                                       ┌──────▼───────┐
                                       │    paste     │ ─▶ pasteboard + ⌘V
                                       └──────────────┘
```

### 1. Detecting copies

macOS offers no "clipboard changed" notification. Every pasteboard has an integer `changeCount` that increases on each copy, so MacClip checks that integer every **300 ms** on the main run loop. Reading one integer is essentially free. Only when it changes does MacClip read the content, so you are not prompted or slowed down needlessly. New content therefore appears in history well under a second after you copy it.

### 2. Capturing content

For each change MacClip builds one entry holding **all representations** of that copy: plain text, RTF, HTML and image data. That is why pasting a rich-text entry back into Pages or Mail keeps its formatting, while pasting into a plain editor gives plain text.

### 3. Filtering for privacy

Before anything is stored, the entry passes the privacy filter:

1. Does the pasteboard carry a concealed / transient / auto-generated marker? → drop.
2. Is the frontmost app on the exclusion list? → drop.
3. Is it over the size cap? → drop and log.
4. Is capture paused? → drop.
5. Was this write made by MacClip itself (see paste flow)? → drop.

### 4. Storing and de-duplicating

Each entry gets a **SHA-256 hash** of its canonical content (text bytes, or image bytes). The hash is unique in the database:

- New hash → insert as newest.
- Existing hash → just update its timestamp, moving it to the top.

Insert and trim happen in a **single SQLite transaction**, keeping at most 20 rows, so a crash can never leave the history in a half-updated state. Images are saved as PNG files with a 96 px thumbnail; when a row is evicted, its files are deleted.

### 5. Showing the panel

The panel is a borderless, **non-activating** window. That detail is the heart of the "paste into the focused field" feature: because MacClip never becomes the active app, the field you were typing in keeps its caret and focus.

- Opened by Option + V → centered on the screen containing the mouse pointer.
- Opened by the menu-bar icon → anchored directly beneath the icon.
- MacClip records which app was frontmost when the panel opened.

### 6. Pasting

When you choose an entry:

1. All its stored representations are written back to the system pasteboard.
2. MacClip remembers the resulting `changeCount` so the monitor ignores its own write.
3. The panel closes; the previous app is re-activated if focus drifted.
4. About 50 ms later MacClip posts a synthetic **⌘V** key press (needs Accessibility).
5. The entry moves to the top of the history and is now your current clipboard.

If Accessibility is not granted, steps 1–3 and 5 still happen and you get a notification asking you to press ⌘V.

### 7. Option + V

The shortcut is registered with the Carbon `RegisterEventHotKey` API, the same mechanism used by many menu-bar apps. It needs no special permission. If another program already owns Option + V, the menu shows "shortcut unavailable" and the icon still works.

### 8. Starting at login

MacClip registers itself with `SMAppService.mainApp` (macOS 13+). It shows up in **System Settings → General → Login Items**, where you can also disable it. If macOS requires approval, MacClip points you there. A single-instance guard prevents duplicate icons.

---

## Where your data lives

| Path | Content |
|---|---|
| `~/Library/Application Support/MacClip/history.db` | SQLite metadata and text/RTF/HTML |
| `~/Library/Application Support/MacClip/images/` | Full-size PNGs |
| `~/Library/Application Support/MacClip/thumbs/` | 96 px thumbnails |
| `~/Library/Application Support/MacClip/config.json` | Settings |
| `~/Library/Logs/MacClip/macclip.log` | Diagnostics (no clipboard contents) |

To remove everything, quit MacClip, disable the login item, delete the app and the two folders above.

---

## Configuration

`config.json` is created with defaults on first run:

```json
{
  "poll_interval_ms": 300,
  "max_text_bytes": 1048576,
  "max_image_bytes": 26214400,
  "excluded_bundle_ids": [
    "com.1password.1password",
    "com.agilebits.onepassword7",
    "com.bitwarden.desktop",
    "com.apple.keychainaccess",
    "com.dashlane.dashlanephonefinal"
  ]
}
```

The history size (20) and shortcut (Option + V) are fixed in v1. Restart MacClip after editing the file.

---

## Building and installing

**Requirements:** macOS 13 or later, Xcode Command Line Tools (`xcode-select --install`) and Rust.

```bash
# 1. Install Rust (once): https://rustup.rs
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"

# 2. Build, install to /Applications and launch (universal arm64 + x86_64)
./scripts/build_app.sh --install
```

Other build modes:

```bash
./scripts/build_app.sh           # build dist/MacClip.app only
./scripts/build_app.sh --host    # host architecture only (faster while developing)
```

The script adds the two Rust targets, builds both, merges them with `lipo`, assembles the bundle (`Info.plist`, icon), and ad-hoc signs it as `com.macclip.app`.

Developer commands: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.

### First launch

1. The MacClip icon appears in the menu bar. There is no Dock icon.
2. MacClip registers itself as a login item so it starts automatically after every login.
3. A dialog explains the Accessibility permission. Click **Continue**, then enable **MacClip** in System Settings → Privacy & Security → Accessibility.
4. Copy something, press **Option + V**, pick an entry.

**About signing:** the app is ad-hoc signed, not notarized. If macOS blocks the first open, right-click the app → Open. After rebuilding, macOS may treat the app as new and ask for Accessibility again; remove the old MacClip entry in System Settings and re-add it.

### Sharing MacClip with others

```bash
./scripts/make_dmg.sh
```

This creates `dist/MacClip-<version>.dmg` (drag-to-Applications installer with a "READ ME FIRST" file) and a `.zip` of the same app. Upload either file anywhere people can download it.

For the person installing: open the DMG and drag **MacClip** to **Applications**, or simply double-click MacClip, which offers to **Move to Applications** and reopens itself from there.

**Gatekeeper warning.** Without a paid Apple Developer account the app cannot be notarized, so macOS shows "cannot be opened / could not be verified" on first launch. Recipients fix it once with System Settings → Privacy & Security → **Open Anyway**, or by running `xattr -dr com.apple.quarantine /Applications/MacClip.app`. This is also written in the DMG's READ ME FIRST file.

To remove the warning entirely, join the Apple Developer Program, then:

```bash
SIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)" \
NOTARY_PROFILE="your-notarytool-profile" \
./scripts/make_dmg.sh
```

(Signing and notarization options are in the scripts but have not been exercised here, as they need your Apple account.)

### Verification checklist

Automatically verified (unit tests and a built-in self-test): 20-item limit, de-duplication, persistence, file permissions, concealed-copy filtering, own-write handling, search, key handling, panel focus behavior, menu contents, universal binary and signature.

Needs you to check once (requires your permissions and keyboard):

- [ ] Option + V opens the panel from another app (e.g. TextEdit) and the app keeps focus.
- [ ] After granting Accessibility, selecting an entry (and number keys 1–9) pastes into the focused field.
- [ ] Rich text and images paste correctly (TextEdit/Notes/Preview).
- [ ] Clicking outside the panel or switching Space closes it.
- [ ] After logging out and in, the icon is back and **MacClip** is listed in System Settings → General → Login Items.

### Developer self-test

Launching the binary with `MACCLIP_SELFTEST=<dir>` makes it open the panel, press keys, choose an entry and write `result.txt` plus panel PNG snapshots into `<dir>`. It needs no permissions and never runs unless that variable is set.

---

## Project layout

```
mac-clipboard/
├── src/
│   ├── lib.rs             platform-independent core (unit tested)
│   ├── config.rs          config.json and data/log paths
│   ├── privacy.rs         concealed / excluded-app / size filtering
│   ├── store.rs           SQLite history, dedupe, trim, search, image files
│   ├── monitor.rs         changeCount tracking: own writes, pause
│   ├── main.rs            bootstrap, single instance, logging, hotkey
│   ├── app.rs             application state, polling, paste flow, self-test
│   ├── pasteboard.rs      NSPasteboard read/write, PNG + thumbnails
│   ├── paste.rs           synthetic ⌘V and notifications
│   ├── permissions.rs     Accessibility checks and prompt
│   ├── autostart.rs       login item via SMAppService
│   └── ui/                status_item.rs (icon, menu), panel.rs (history panel)
├── scripts/               build_app.sh, make_icons.sh, make_icon.py
├── packaging/Info.plist   bundle metadata
├── assets/                generated app icon
├── docs/index.html        HTML documentation
└── openspec/              requirements, design and task list
```

---

## Limitations and troubleshooting

| Symptom | Cause / fix |
|---|---|
| Selecting an entry only copies, does not paste | Accessibility not granted. Enable MacClip in System Settings → Privacy & Security → Accessibility. |
| Pasting stopped working after a rebuild | The ad-hoc signature changed. Remove and re-add MacClip in Accessibility. |
| Option + V types "√" | The shortcut could not be registered. Check the menu for "shortcut unavailable" and close the conflicting app. |
| Nothing pastes into a password field | Secure input fields block synthetic key presses by design. Press ⌘V yourself; the entry is already on the clipboard. |
| A password I copied is in history | The source app did not flag it as concealed. Use Pause or add the app to `excluded_bundle_ids`. |
| Files copied in Finder are not listed | File entries are not supported in v1. |
| No menu-bar icon | The menu bar may be crowded (notch / many icons). Quit some other menu-bar apps or hide them. |

---

## Development workflow

The project uses [OpenSpec](https://github.com/Fission-AI/OpenSpec) for spec-driven development with Claude Code and Antigravity.

- Requirements: `openspec/changes/macclip-clipboard-manager/specs/`
- Design decisions: `openspec/changes/macclip-clipboard-manager/design.md`
- Task list: `openspec/changes/macclip-clipboard-manager/tasks.md`
- Implement with `/opsx:apply` (Claude Code) or `/opsx-apply` (Antigravity).

---

MacClip — Clipboard history for macOS.
