#!/bin/sh
# Generate assets/MacClip.icns from the procedurally drawn master icon.
set -eu
cd "$(dirname "$0")/.."
mkdir -p assets
SET=$(mktemp -d)/MacClip.iconset
mkdir -p "$SET"
python3 scripts/make_icon.py assets/MacClip-1024.png
for s in 16 32 128 256 512; do
  sips -z $s $s assets/MacClip-1024.png --out "$SET/icon_${s}x${s}.png" >/dev/null
  d=$((s * 2))
  sips -z $d $d assets/MacClip-1024.png --out "$SET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o assets/MacClip.icns
echo "Wrote assets/MacClip.icns"
