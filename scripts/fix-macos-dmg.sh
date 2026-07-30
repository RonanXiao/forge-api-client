#!/usr/bin/env bash
# Hide .VolumeIcon.icns inside the built DMG so Finder doesn't show it
# as a third "file" next to Forge.app and Applications.
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

# Prefer the latest non-temp DMG
DMG="$(ls -t "${DMGS[@]}" | head -1)"
echo "fix-macos-dmg: processing $DMG"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/fp-dmg.XXXXXX")"
RW_DMG="$TMP_DIR/rw.dmg"
MOUNT_POINT="$TMP_DIR/mount"
mkdir -p "$MOUNT_POINT"

cleanup() {
  if mount | grep -q "$MOUNT_POINT"; then
    hdiutil detach "$MOUNT_POINT" -quiet -force 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# UDZO (compressed) → UDRW so we can edit, then convert back
hdiutil convert "$DMG" -format UDRW -o "$RW_DMG" -quiet
# Attach without mounting into /Volumes with a pretty name conflict
DEVICE="$(hdiutil attach -readwrite -noverify -noautoopen "$RW_DMG" -mountpoint "$MOUNT_POINT" \
  | awk '/^\/dev\// { print $1; exit }')"

ICON="$MOUNT_POINT/.VolumeIcon.icns"
if [[ -f "$ICON" ]]; then
  # Finder invisible attribute (SetFile) + BSD hidden flag
  if command -v SetFile >/dev/null 2>&1; then
    SetFile -a V "$ICON" || true
    SetFile -c icnC "$ICON" || true
  fi
  chflags hidden "$ICON" 2>/dev/null || true
  echo "fix-macos-dmg: hid .VolumeIcon.icns"
else
  echo "fix-macos-dmg: no .VolumeIcon.icns present (ok)"
fi

# Detach cleanly
hdiutil detach "$MOUNT_POINT" -quiet -force
# Overwrite original with compressed image
hdiutil convert "$RW_DMG" -format UDZO -imagekey zlib-level=9 -o "$TMP_DIR/out.dmg" -quiet
mv -f "$TMP_DIR/out.dmg" "$DMG"
echo "fix-macos-dmg: wrote $DMG"
