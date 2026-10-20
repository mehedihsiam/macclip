#!/bin/sh
# MacClip installer: downloads the latest release and installs it to Applications.
# Usage:  curl -fsSL https://raw.githubusercontent.com/mehedihsiam/macclip/main/install.sh | sh
set -eu

URL="https://github.com/mehedihsiam/macclip/releases/latest/download/MacClip.dmg"

[ "$(uname -s)" = "Darwin" ] || { echo "MacClip is for macOS only." >&2; exit 1; }

TMP=$(mktemp -d)
MNT="$TMP/mnt"
cleanup() {
  hdiutil detach "$MNT" -quiet 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT

echo "Downloading MacClip..."
curl -fsSL -o "$TMP/MacClip.dmg" "$URL"

mkdir -p "$MNT"
hdiutil attach "$TMP/MacClip.dmg" -nobrowse -readonly -mountpoint "$MNT" -quiet

# Close a running copy so it can be replaced.
osascript -e 'tell application id "com.macclip.app" to quit' >/dev/null 2>&1 || true
sleep 1

DEST="/Applications"
if ! { rm -rf "$DEST/MacClip.app" && ditto "$MNT/MacClip.app" "$DEST/MacClip.app"; } 2>/dev/null; then
  DEST="$HOME/Applications"
  mkdir -p "$DEST"
  rm -rf "$DEST/MacClip.app"
  ditto "$MNT/MacClip.app" "$DEST/MacClip.app"
fi
xattr -dr com.apple.quarantine "$DEST/MacClip.app" 2>/dev/null || true

echo "Installed MacClip to $DEST."
open "$DEST/MacClip.app"
echo "Done. Look for the MacClip icon in your menu bar, and press Option+V."
