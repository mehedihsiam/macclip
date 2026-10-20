#!/bin/sh
# Build a shareable disk image (and zip) containing MacClip.app.
#   ./scripts/make_dmg.sh
# Optional environment:
#   SIGN_IDENTITY   Developer ID Application identity (see build_app.sh)
#   NOTARY_PROFILE  notarytool keychain profile; if set, the DMG is notarized
#                   and stapled (requires SIGN_IDENTITY)
set -eu
cd "$(dirname "$0")/.."

./scripts/build_app.sh

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DMG="dist/MacClip-$VERSION.dmg"
ZIP="dist/MacClip-$VERSION.zip"
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT

cp -R dist/MacClip.app "$STAGE/MacClip.app"
ln -s /Applications "$STAGE/Applications"
cp packaging/READ-ME-FIRST.txt "$STAGE/READ ME FIRST.txt"

rm -f "$DMG" "$ZIP"
hdiutil create -volname "MacClip" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
[ "${SIGN_IDENTITY:--}" != "-" ] && codesign --force --sign "$SIGN_IDENTITY" --timestamp "$DMG"

if [ -n "${NOTARY_PROFILE:-}" ]; then
  xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
  xcrun stapler staple "$DMG"
fi

ditto -c -k --keepParent dist/MacClip.app "$ZIP"
# Stable file names so ".../releases/latest/download/MacClip.dmg" always works.
cp "$DMG" dist/MacClip.dmg
cp "$ZIP" dist/MacClip.zip
echo "Created:"
ls -lh "$DMG" "$ZIP" dist/MacClip.dmg dist/MacClip.zip
