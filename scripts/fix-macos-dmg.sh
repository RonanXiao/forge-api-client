#!/usr/bin/env bash
# Rebuild a clean macOS DMG with ONLY:
#   - Forge.app
#   - Applications (symlink)
# Tauri/create-dmg always injects .VolumeIcon.icns for a custom volume icon.
# Hiding/deleting it is unreliable (Finder ghosts / .DS_Store). So we discard
# Tauri's DMG and recreate from the .app with plain hdiutil (no volicon).
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "skip fix-macos-dmg: not macOS"
  exit 0
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/src-tauri/target/release/bundle/macos/Forge.app"
DMG_DIR="$ROOT/src-tauri/target/release/bundle/dmg"
PRODUCT_NAME="Forge"
VERSION="$(
  python3 -c "import json;print(json.load(open('$ROOT/src-tauri/tauri.conf.json'))['version'])" 2>/dev/null \
    || echo "0.1.0"
)"
ARCH="$(uname -m)"
# match tauri naming: Forge_0.1.0_aarch64.dmg
case "$ARCH" in
  arm64) ARCH_LABEL="aarch64" ;;
  x86_64) ARCH_LABEL="x64" ;;
  *) ARCH_LABEL="$ARCH" ;;
esac
OUT_DMG="$DMG_DIR/${PRODUCT_NAME}_${VERSION}_${ARCH_LABEL}.dmg"

if [[ ! -d "$APP" ]]; then
  echo "skip fix-macos-dmg: missing $APP (build app bundle first)"
  exit 0
fi

mkdir -p "$DMG_DIR"
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/forge-dmg-stage.XXXXXX")"
RW_DMG="$STAGE/rw.dmg"
MNT="$STAGE/mnt"
CLEAN_SRC="$STAGE/src"
mkdir -p "$CLEAN_SRC" "$MNT"

cleanup() {
  if mount | grep -q "$MNT"; then
    hdiutil detach "$MNT" -quiet -force 2>/dev/null || true
  fi
  # eject anything left from failed runs
  hdiutil detach "$MNT" -quiet -force 2>/dev/null || true
  rm -rf "$STAGE"
}
trap cleanup EXIT

echo "fix-macos-dmg: staging clean contents from $APP"
# ditto preserves resource forks / codesign better than cp -R
ditto "$APP" "$CLEAN_SRC/${PRODUCT_NAME}.app"
ln -s /Applications "$CLEAN_SRC/Applications"

# Remove any stale dmg(s) from tauri so the user can't open the wrong one
rm -f "$DMG_DIR"/*.dmg

# Create uncompressed RW image from folder (no volume icon file)
echo "fix-macos-dmg: creating image..."
hdiutil create \
  -volname "$PRODUCT_NAME" \
  -srcfolder "$CLEAN_SRC" \
  -ov \
  -format UDRW \
  -fs HFS+ \
  "$RW_DMG" >/dev/null

# Mount and tidy Finder window: only two icons, no ghosts
hdiutil attach -readwrite -noverify -noautoopen "$RW_DMG" -mountpoint "$MNT" >/dev/null

# Belt-and-suspenders: never leave volume icon artifacts
rm -f "$MNT/.VolumeIcon.icns" "$MNT/.DS_Store"
rm -rf "$MNT/.fseventsd" "$MNT/.Spotlight-V100" "$MNT/.Trashes"
if command -v SetFile >/dev/null 2>&1; then
  SetFile -a c "$MNT" 2>/dev/null || true
fi

# Position icons via AppleScript (app left, Applications right)
if command -v osascript >/dev/null 2>&1; then
  osascript <<EOF || true
tell application "Finder"
  tell disk "$PRODUCT_NAME"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set the bounds of container window to {200, 120, 860, 520}
    set viewOptions to the icon view options of container window
    set arrangement of viewOptions to not arranged
    set icon size of viewOptions to 128
    set position of item "${PRODUCT_NAME}.app" of container window to {180, 170}
    set position of item "Applications" of container window to {480, 170}
    update without registering applications
    delay 0.5
    close
  end tell
end tell
EOF
fi

# Final sweep after AppleScript (it may recreate .DS_Store — keep it for positions)
rm -f "$MNT/.VolumeIcon.icns"
if command -v SetFile >/dev/null 2>&1; then
  SetFile -a c "$MNT" 2>/dev/null || true
fi

sync
hdiutil detach "$MNT" -quiet -force

echo "fix-macos-dmg: compressing $OUT_DMG"
hdiutil convert "$RW_DMG" -format UDZO -imagekey zlib-level=9 -o "$OUT_DMG" -quiet

# Verify
VERIFY_MNT="$STAGE/verify"
mkdir -p "$VERIFY_MNT"
hdiutil attach -readonly -noverify -noautoopen "$OUT_DMG" -mountpoint "$VERIFY_MNT" >/dev/null
echo "fix-macos-dmg: final contents:"
ls -la "$VERIFY_MNT"
if [[ -e "$VERIFY_MNT/.VolumeIcon.icns" ]]; then
  echo "ERROR: .VolumeIcon.icns still present after rebuild" >&2
  hdiutil detach "$VERIFY_MNT" -quiet -force || true
  exit 1
fi
# ensure only app + Applications (+ optional .DS_Store)
ENTRIES="$(find "$VERIFY_MNT" -maxdepth 1 ! -path "$VERIFY_MNT" ! -name '.DS_Store' ! -name '.fseventsd' -print | wc -l | tr -d ' ')"
hdiutil detach "$VERIFY_MNT" -quiet -force
echo "fix-macos-dmg: visible entry count (excl. DS_Store)=$ENTRIES"
echo "fix-macos-dmg: done → $OUT_DMG"
echo ""
echo "IMPORTANT: Eject any already-open \"Forge\" volume in Finder, then open:"
echo "  $OUT_DMG"
