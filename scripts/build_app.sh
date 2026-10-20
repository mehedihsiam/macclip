#!/bin/sh
# Build MacClip.app (universal arm64 + x86_64, ad-hoc signed).
#   ./scripts/build_app.sh            build dist/MacClip.app
#   ./scripts/build_app.sh --install  also install to /Applications and launch
#   ./scripts/build_app.sh --host     host architecture only (faster, for development)
set -eu
cd "$(dirname "$0")/.."
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

INSTALL=0; HOST_ONLY=0
for a in "$@"; do
  case "$a" in
    --install) INSTALL=1 ;;
    --host) HOST_ONLY=1 ;;
    *) echo "unknown option: $a" >&2; exit 2 ;;
  esac
done

command -v cargo >/dev/null || { echo "Rust not found. Install it from https://rustup.rs" >&2; exit 1; }

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APP=dist/MacClip.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

if [ "$HOST_ONLY" = 1 ]; then
  cargo build --release
  cp target/release/macclip "$APP/Contents/MacOS/MacClip"
else
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  cargo build --release --target aarch64-apple-darwin
  cargo build --release --target x86_64-apple-darwin
  lipo -create \
    target/aarch64-apple-darwin/release/macclip \
    target/x86_64-apple-darwin/release/macclip \
    -output "$APP/Contents/MacOS/MacClip"
fi

sed "s/__VERSION__/$VERSION/g" packaging/Info.plist > "$APP/Contents/Info.plist"
[ -f assets/MacClip.icns ] || ./scripts/make_icons.sh
cp assets/MacClip.icns "$APP/Contents/Resources/MacClip.icns"

# SIGN_IDENTITY="Developer ID Application: Name (TEAMID)" enables a distributable,
# notarizable signature; the default "-" is an ad-hoc signature.
SIGN_IDENTITY="${SIGN_IDENTITY:--}"
if [ "$SIGN_IDENTITY" = "-" ]; then
  codesign --force --deep --sign - --identifier com.macclip.app "$APP"
else
  codesign --force --deep --options runtime --timestamp --sign "$SIGN_IDENTITY" --identifier com.macclip.app "$APP"
fi
codesign --verify --deep --strict "$APP"
echo "Built $APP ($(lipo -archs "$APP/Contents/MacOS/MacClip"))"

if [ "$INSTALL" = 1 ]; then
  pkill -x MacClip 2>/dev/null || true
  sleep 0.5
  rm -rf /Applications/MacClip.app
  ditto "$APP" /Applications/MacClip.app
  open /Applications/MacClip.app
  echo "Installed to /Applications/MacClip.app and launched."
fi
