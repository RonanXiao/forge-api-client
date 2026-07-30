#!/usr/bin/env bash
# Remove .VolumeIcon.icns from the built DMG.
# Tauri/create-dmg drops it for a custom volume icon; Finder still shows it
# (especially with "show hidden files"), so we delete it entirely.
# Window contents should only be: Forge.app + Applications.
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "skip fix-macos-dmg: not macOS"
  exit 0
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DMG_DIR="$ROOT/src-tauri/target/release/bundle/dmg"

shopt -s nullglob
DMGS=("$DMG_DIR"/*.dmg)
if [[ ${#DMGS[@]} -eq 0 ]]; then
  echo "skip fix-macos-dmg: no .dmg under $DMG_DIR"
  exit 0
fi

DMG="$(ls -t "${DMGS[@]}" | head -1)"
echo "fix-macos-dmg: processing $DMG"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/forge-dmg.XXXXXX")"
RW_DMG="$TMP_DIR/rw.dmg"
MOUNT_POINT="$TMP_DIR/mount"
mkdir -p "$MOUNT_POINT"

cleanup() {
  if mount | grep -q "$MOUNT_POINT"; then
    hdiutil detach "$MOUNT_POINT" -quiet -force 2>/dev/null || true
  fi
  # convert may leave .dmg in TMP_DIR
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# Compressed UDZO → read-write
hdiutil convert "$DMG" -format UDRW -o "$RW_DMG" -quiet
hdiutil attach -readwrite -noverify -noautoopen "$RW_DMG" -mountpoint "$MOUNT_POINT" >/dev/null

ICON="$MOUNT_POINT/.VolumeIcon.icns"
if [[ -e "$ICON" ]]; then
  # Clear custom-icon bit on the volume first (so Finder stops looking for the file)
  if command -v SetFile >/dev/null 2>&1; then
    SetFile -a c "$MOUNT_POINT" 2>/dev/null || true
  fi
  chflags nouchg,noschg "$ICON" 2>/dev/null || true
  rm -f "$ICON"
  echo "fix-macos-dmg: removed .VolumeIcon.icns"
else
  echo "fix-macos-dmg: no .VolumeIcon.icns present (ok)"
fi

# Drop Finder junk that can re-surface ghost icons
rm -rf "$MOUNT_POINT/.fseventsd" 2>/dev/null || true
rm -f "$MOUNT_POINT/.DS_Store" 2>/dev/null || true

sync
hdiutil detach "$MOUNT_POINT" -quiet -force

hdiutil convert "$RW_DMG" -format UDZO -imagekey zlib-level=9 -o "$TMP_DIR/out.dmg" -quiet
mv -f "$TMP_DIR/out.dmg" "$DMG"
echo "fix-macos-dmg: wrote $DMG"
