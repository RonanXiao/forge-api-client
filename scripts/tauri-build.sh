#!/usr/bin/env bash
# Cross-platform release build wrapper.
# On macOS: do NOT let Tauri produce/open a DMG (it injects .VolumeIcon.icns
# and auto-opens that broken image). Build .app only, then make a clean DMG
# and open that instead.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" == "Darwin" ]]; then
  echo "tauri-build: macOS → bundles=app only (clean DMG made next)"
  # Extra args after -- are forwarded, e.g. pnpm tauri:build -- --verbose
  pnpm exec tauri build --bundles app "$@"

  # Bundle forge-cli inside the .app (not on PATH until user installs via menu)
  APP="$ROOT/src-tauri/target/release/bundle/macos/Forge.app"
  CLI="$ROOT/src-tauri/target/release/forge-cli"
  if [[ -x "$CLI" && -d "$APP" ]]; then
    DEST="$APP/Contents/Resources/bin"
    mkdir -p "$DEST"
    cp -f "$CLI" "$DEST/forge-cli"
    chmod +x "$DEST/forge-cli"
    echo "tauri-build: bundled forge-cli → Contents/Resources/bin/forge-cli"
  else
    echo "tauri-build: warn — forge-cli not found at $CLI (build both bins)"
    # Ensure CLI is built even if tauri only rebuilt the GUI binary
    (cd "$ROOT/src-tauri" && cargo build --release --bin forge-cli)
    if [[ -x "$CLI" && -d "$APP" ]]; then
      DEST="$APP/Contents/Resources/bin"
      mkdir -p "$DEST"
      cp -f "$CLI" "$DEST/forge-cli"
      chmod +x "$DEST/forge-cli"
      echo "tauri-build: bundled forge-cli → Contents/Resources/bin/forge-cli"
    fi
  fi

  bash "$ROOT/scripts/fix-macos-dmg.sh"
else
  echo "tauri-build: non-macOS → full tauri build"
  pnpm exec tauri build "$@"
fi
