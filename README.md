<p align="center">
  <img src="docs/icon.png" width="112" alt="MacClip icon">
</p>

<h1 align="center">MacClip</h1>
<p align="center"><strong>Clipboard history for macOS</strong><br>
Copy as usual. Press <kbd>Option</kbd> + <kbd>V</kbd> to paste anything you copied recently.</p>

<p align="center">
  <a href="https://github.com/mehedihsiam/macclip/releases/latest/download/MacClip.dmg"><strong>⬇ Download MacClip for Mac</strong></a><br>
  <sub>Free · macOS 13 (Ventura) or newer · Intel and Apple Silicon</sub>
</p>

---

## What MacClip does

Your Mac normally remembers only the last thing you copied. MacClip quietly remembers your **last 20 copies** (text, formatted text and pictures) and lets you bring any of them back in one step.

- Lives in the menu bar at the top right of your screen. No Dock icon, no windows in your way.
- Starts automatically when you log in.
- Keeps everything private on your own Mac.

## Install

There are three ways. Pick whichever you prefer.

### Option A: One command (no warning from macOS)

1. Open the **Terminal** app (press <kbd>⌘</kbd> + <kbd>Space</kbd>, type "Terminal", press <kbd>Return</kbd>).
2. Paste this line and press <kbd>Return</kbd>:

```
curl -fsSL https://raw.githubusercontent.com/mehedihsiam/macclip/main/install.sh | sh
```

MacClip downloads, installs into your Applications folder and opens. macOS shows no security warning with this method.

### Option B: Homebrew

If you use [Homebrew](https://brew.sh), run this in Terminal:

```
brew install --cask mehedihsiam/macclip/macclip
```

To update later, run `brew upgrade --cask macclip`. To remove MacClip, run `brew uninstall --cask macclip`.

### Option C: Download the app

1. **[Download MacClip](https://github.com/mehedihsiam/macclip/releases/latest/download/MacClip.dmg)** (the `.dmg` file).
2. Double-click the downloaded file to open it.
3. Drag **MacClip** onto the **Applications** folder.
   *Prefer to skip dragging? Double-click MacClip instead and choose **Move to Applications**.*
4. Open **MacClip** from your Applications folder.

**macOS may say it "could not verify MacClip is free of malware."** This appears for any app that its author hasn't registered with Apple (which costs a yearly fee). MacClip is open source and never connects to the internet, so you can allow it once:

1. Try to open MacClip. When the warning appears, click **Done**.
2. Open **System Settings → Privacy & Security**.
3. Scroll down to the message about MacClip and click **Open Anyway**.
4. Confirm with your password or Touch ID. MacClip opens, and you won't be asked again.

## First-time setup

When MacClip opens, it checks what it needs. If something is missing, it shows a short window explaining **what** it needs and **why**, with an **Allow Access…** button.

**Accessibility** lets MacClip paste for you, so the item you choose appears right where you were typing.

1. Click **Allow Access…**
2. In *System Settings → Privacy & Security → Accessibility*, switch **MacClip** on.
3. MacClip notices within a few seconds and tells you it is ready. No restart needed.

- **Already listed but not working** (for example after an update)? Select MacClip in the list, click **–** to remove it, then add it again with **+**.
- **Prefer not to?** MacClip still works. When you pick an item it is copied, and you press <kbd>⌘</kbd> + <kbd>V</kbd> yourself. You can tick **Don't remind me again** in that window.
- On newer versions of macOS, if *Paste from Other Apps* is set to "Don't Allow" for MacClip, the same window will explain how to allow it.

**It starts by itself.** MacClip adds itself to your login items the first time it runs, so it opens automatically every time you log in. You never need to open it again. If macOS asks you to approve it, MacClip shows you where (*System Settings → General → Login Items*). To turn this off, right-click the menu-bar icon and untick **Launch at login**.

MacClip never reads what you type and never sends anything anywhere.

---

## How to use it

### 1. Copy things as you normally do
Use <kbd>⌘</kbd> + <kbd>C</kbd> as always. MacClip remembers each item.

### 2. Open your history
Press <kbd>Option</kbd> + <kbd>V</kbd> anywhere, or click the MacClip icon in the menu bar.

### 3. Pick what you want

| To do this | Do this |
|---|---|
| Move through the list | <kbd>↑</kbd> <kbd>↓</kbd> |
| Paste the highlighted item | <kbd>Return</kbd> |
| Paste one of the first nine instantly | Press its number <kbd>1</kbd> – <kbd>9</kbd> |
| Paste with the mouse | Click the item |
| Find something | Just start typing |
| Fix a typo in your search | <kbd>Delete</kbd> |
| Close without pasting | <kbd>Esc</kbd> or click anywhere else |

The item appears where your cursor was, in the app you were using.

### The menu-bar icon
**Right-click** (or Control-click) the icon for options:

- **Pause capture / Resume capture**: stop remembering for a while.
- **Clear history**: delete everything MacClip has saved.
- **Launch at login**: turn automatic start on or off.
- **Accessibility**: shows whether automatic pasting is on.
- **Quit MacClip**

## Good to know

- MacClip keeps the **20 most recent** items. Copying something you already copied simply moves it back to the top.
- It remembers items even after you restart your Mac.
- **Passwords are protected.** Items that password managers mark as secret are never saved, and copies made inside 1Password, Bitwarden, Dashlane and Keychain Access are ignored.
- Very large items (over 1 MB of text or 25 MB of images) are skipped.
- If you are about to copy something sensitive from another app, choose **Pause capture** first.

## Troubleshooting

| What you see | What to do |
|---|---|
| Picking an item copies it but doesn't paste | Turn on MacClip in *System Settings → Privacy & Security → Accessibility*. Until then, press <kbd>⌘</kbd> + <kbd>V</kbd>. |
| Pasting stopped working after an update | In the Accessibility list, remove MacClip (the **–** button), then add it again with **+**. |
| <kbd>Option</kbd> + <kbd>V</kbd> types "√" or does nothing | Another app is using that shortcut. Quit it, or open MacClip from its menu-bar icon instead. The menu will say "Shortcut unavailable". |
| I can't paste into a password box | Some secure fields block automatic pasting. The item is already copied, so press <kbd>⌘</kbd> + <kbd>V</kbd>. |
| A password I copied is in the list | Not every app marks passwords as secret. Use **Clear history**, and **Pause capture** before copying secrets. |
| I can't see the icon | Your menu bar may be full. Close or hide some menu-bar apps. |
| Files copied in Finder aren't listed | MacClip remembers text and pictures only. |

## Uninstall

1. Right-click the MacClip icon and choose **Quit MacClip**.
2. Drag **MacClip** from the Applications folder to the Trash.
3. Optional: remove saved history by deleting the folder `MacClip` inside `~/Library/Application Support`.

## Help and feedback

Found a problem or have an idea? [Open an issue](https://github.com/mehedihsiam/macclip/issues) and tell us what happened.

<sub>Developers: see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).</sub>
