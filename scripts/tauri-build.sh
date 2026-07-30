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
  bash "$ROOT/scripts/fix-macos-dmg.sh"
else
  echo "tauri-build: non-macOS → full tauri build"
  pnpm exec tauri build "$@"
fi
