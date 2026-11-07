#!/bin/bash
# Builds Folio.app (Tauri + Rust) for Apple Silicon.
#   bash scripts/build.sh            -> ../deliverables/Folio.app (+ DMG with --dmg)
#   FOLIO_OUTPUT=/some/dir bash scripts/build.sh
# The Tauri CLI is not required: the Rust crate embeds ../web at compile time and
# this script assembles the .app bundle itself.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
OUTPUT="${FOLIO_OUTPUT:-$ROOT/../deliverables}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' src-tauri/Cargo.toml | head -1)"
APP="$OUTPUT/Folio.app"

( cd src-tauri && cargo build --release ${CARGO_OFFLINE:+--offline} )

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp src-tauri/target/release/folio "$APP/Contents/MacOS/Folio"
cp src-tauri/icons/icon.icns "$APP/Contents/Resources/Folio.icns"
sed "s/__VERSION__/$VERSION/g" scripts/Info.plist > "$APP/Contents/Info.plist"
# Apple Silicon needs at least an ad-hoc signature. No Developer ID / notarization.
codesign --force --deep --sign - "$APP"
echo "Built $APP ($VERSION)"

if [ "${1:-}" = "--dmg" ]; then
  # A styled installer window: dark background art (rendered by scripts/dmg-art with
  # CoreGraphics/CoreText), Folio on the left, Applications on the right.
  # The icon positions below must match LEFT/RIGHT in scripts/dmg-art/src/main.rs.
  DMG="$OUTPUT/Folio-$VERSION-Apple-Silicon.dmg"
  WORK="$ROOT/.build/dmg"; STAGING="$WORK/staging"; RW="$WORK/Folio-rw.dmg"; MNT="/Volumes/Folio"
  if [ -d "$MNT" ]; then
    echo "error: $MNT is already mounted; eject it first" >&2; exit 1
  fi
  rm -rf "$WORK"; mkdir -p "$STAGING/.background"

  ( cd scripts/dmg-art && CARGO_TARGET_DIR="$ROOT/.build/dmg-art-target" cargo build --release --quiet ${CARGO_OFFLINE:+--offline} )
  "$ROOT/.build/dmg-art-target/release/dmg-art" "$WORK/art" "$VERSION"
  tiffutil -cathidpicheck "$WORK/art/background.png" "$WORK/art/background@2x.png" -out "$STAGING/.background/background.tiff"

  ditto "$APP" "$STAGING/Folio.app"
  ln -sfn /Applications "$STAGING/Applications"

  hdiutil create -quiet -volname Folio -srcfolder "$STAGING" -fs HFS+ -format UDRW -size 64m -ov "$RW"
  hdiutil attach -quiet -readwrite -noverify -noautoopen -mountpoint "$MNT" "$RW"
  SetFile -a V "$MNT/.background"

  osascript <<'APPLESCRIPT'
tell application "Finder"
  tell disk "Folio"
    open
    set w to container window
    set current view of w to icon view
    set toolbar visible of w to false
    set statusbar visible of w to false
    set pathbar visible of w to false
    set sidebar width of w to 0
    -- 720x440 content; the extra 28pt is the title bar.
    set bounds of w to {240, 140, 960, 608}
    set opts to icon view options of w
    set arrangement of opts to not arranged
    set icon size of opts to 128
    set text size of opts to 13
    set shows item info of opts to false
    set shows icon preview of opts to false
    set background picture of opts to file ".background:background.tiff"
    set position of item "Folio.app" of w to {190, 232}
    set position of item "Applications" of w to {530, 232}
    update without registering applications
    delay 1
    close
    open
    delay 2
    close
  end tell
end tell
APPLESCRIPT

  # Volume icon goes in after the Finder pass (Finder drops it otherwise).
  # Slim copy of the app icon (no 512/1024px reps): saves ~1.7 MB in the download.
  iconutil -c iconset src-tauri/icons/icon.icns -o "$WORK/vol.iconset"
  rm -f "$WORK"/vol.iconset/icon_512x512*.png "$WORK"/vol.iconset/icon_256x256.png
  iconutil -c icns "$WORK/vol.iconset" -o "$MNT/.VolumeIcon.icns"
  SetFile -a V "$MNT/.VolumeIcon.icns"
  SetFile -a C "$MNT"
  chmod -Rf go-w "$MNT" || true
  rm -rf "$MNT/.fseventsd" "$MNT/.Trashes"
  sync
  diskutil eject "$MNT" >/dev/null || { sleep 2; diskutil eject force "$MNT" >/dev/null; }
  rm -f "$DMG"
  hdiutil convert -quiet "$RW" -format UDZO -imagekey zlib-level=9 -o "$DMG"
  rm -f "$RW"
  echo "Built $DMG ($(du -h "$DMG" | cut -f1 | xargs))"
fi
