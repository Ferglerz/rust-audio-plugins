#!/usr/bin/env zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export CARGO_TARGET_DIR="$ROOT/target"
export CARGO_INCREMENTAL=0

echo "Bundling release VST3..."
cargo xtask bundle damian-channel-strip --release

BUNDLE="$CARGO_TARGET_DIR/bundled/Damian Channel Strip.vst3"
if [[ ! -d "$BUNDLE" ]]; then
  echo "error: expected bundle at $BUNDLE" >&2
  exit 1
fi

echo "Bundle created at $BUNDLE"
