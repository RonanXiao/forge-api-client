#!/usr/bin/env bash
# Build a clean macOS DMG with ONLY:
#   - Forge.app
#   - Applications (symlink)
#
# Called after `tauri build --bundles app`. Never uses Tauri's DMG (which
# injects .VolumeIcon.icns and auto-opens a messy installer window).
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
case "$ARCH" in
  arm64) ARCH_LABEL="aarch64" ;;
  x86_64) ARCH_LABEL="x64" ;;
  *) ARCH_LABEL="$ARCH" ;;
esac
OUT_DMG="$DMG_DIR/${PRODUCT_NAME}_${VERSION}_${ARCH_LABEL}.dmg"

if [[ ! -d "$APP" ]]; then
  echo "ERROR: missing $APP — run tauri build --bundles app first" >&2
  exit 1
fi

mkdir -p "$DMG_DIR"
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/forge-dmg-stage.XXXXXX")"
RW_DMG="$STAGE/rw.dmg"
MNT="$STAGE/mnt"
CLEAN_SRC="$STAGE/src"
mkdir -p "$CLEAN_SRC" "$MNT"

cleanup() {
  hdiutil detach "$MNT" -quiet -force 2>/dev/null || true
  rm -rf "$STAGE"
}
trap cleanup EXIT

# Eject any already-mounted Forge volumes so Finder won't keep an old window
for vol in /Volumes/"$PRODUCT_NAME" /Volumes/"${PRODUCT_NAME} "*; do
  if [[ -d "$vol" ]]; then
    echo "fix-macos-dmg: ejecting $vol"
    hdiutil detach "$vol" -quiet -force 2>/dev/null || true
  fi
done

echo "fix-macos-dmg: staging from $APP"
ditto "$APP" "$CLEAN_SRC/${PRODUCT_NAME}.app"
ln -s /Applications "$CLEAN_SRC/Applications"

rm -f "$DMG_DIR"/*.dmg

echo "fix-macos-dmg: creating image..."
# UDZO in one shot — no remount → fewer .fseventsd / ghost files
hdiutil create \
  -volname "$PRODUCT_NAME" \
  -srcfolder "$CLEAN_SRC" \
  -ov \
  -format UDZO \
  -imagekey zlib-level=9 \
  -fs HFS+ \
  "$OUT_DMG" >/dev/null

# Optional: mount once RO to hide system junk if present, re-pack if needed
hdiutil attach -readwrite -noverify -noautoopen "$OUT_DMG" -mountpoint "$MNT" >/dev/null 2>&1 || {
  # UDZO may not mount RW — convert path
  hdiutil convert "$OUT_DMG" -format UDRW -o "$RW_DMG" -quiet
  rm -f "$OUT_DMG"
  hdiutil attach -readwrite -noverify -noautoopen "$RW_DMG" -mountpoint "$MNT" >/dev/null
  NEED_CONVERT=1
}

rm -f "$MNT/.VolumeIcon.icns" "$MNT/.DS_Store"
rm -rf "$MNT/.fseventsd" "$MNT/.Spotlight-V100" "$MNT/.Trashes" "$MNT/.TemporaryItems"
if command -v SetFile >/dev/null 2>&1; then
  SetFile -a c "$MNT" 2>/dev/null || true
  # If fseventsd reappears before detach, hide it
  if [[ -d "$MNT/.fseventsd" ]]; then
    SetFile -a V "$MNT/.fseventsd" 2>/dev/null || true
    chflags hidden "$MNT/.fseventsd" 2>/dev/null || true
  fi
fi

sync
hdiutil detach "$MNT" -quiet -force

if [[ "${NEED_CONVERT:-0}" == "1" ]]; then
  hdiutil convert "$RW_DMG" -format UDZO -imagekey zlib-level=9 -o "$OUT_DMG" -quiet
fi

# Verify
VERIFY_MNT="$STAGE/verify"
mkdir -p "$VERIFY_MNT"
hdiutil attach -readonly -noverify -noautoopen "$OUT_DMG" -mountpoint "$VERIFY_MNT" >/dev/null
echo "fix-macos-dmg: final contents:"
ls -la "$VERIFY_MNT"
if [[ -e "$VERIFY_MNT/.VolumeIcon.icns" ]]; then
  echo "ERROR: .VolumeIcon.icns still present" >&2
  hdiutil detach "$VERIFY_MNT" -quiet -force || true
  exit 1
fi
hdiutil detach "$VERIFY_MNT" -quiet -force

echo "fix-macos-dmg: done → $OUT_DMG"
echo "fix-macos-dmg: opening clean installer..."
open "$OUT_DMG"
