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

bash scripts/bundle.sh
( cd src-tauri && cargo build --release ${CARGO_OFFLINE:+--offline} )

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp src-tauri/target/release/folio "$APP/Contents/MacOS/Folio"
cp src-tauri/icons/icon.icns "$APP/Contents/Resources/Folio.icns"
cp -R THIRD_PARTY "$APP/Contents/Resources/"
sed "s/__VERSION__/$VERSION/g" scripts/Info.plist > "$APP/Contents/Info.plist"
# Apple Silicon needs at least an ad-hoc signature. No Developer ID / notarization.
codesign --force --deep --sign - "$APP"
echo "Built $APP ($VERSION)"

if [ "${1:-}" = "--dmg" ]; then
  STAGING="$ROOT/.build/dmg"; rm -rf "$STAGING"; mkdir -p "$STAGING"
  ditto "$APP" "$STAGING/Folio.app"
  ln -sfn /Applications "$STAGING/Applications"
  hdiutil create -volname Folio -srcfolder "$STAGING" -ov -format UDZO "$OUTPUT/Folio-$VERSION-Apple-Silicon.dmg"
fi
