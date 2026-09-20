#!/usr/bin/env zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Always build into this repo's target. Agent shells inject CARGO_TARGET_DIR into a
# sandbox cache; cargo metadata follows that and install would copy stale artifacts.
export CARGO_TARGET_DIR="$ROOT/target"
export CARGO_INCREMENTAL=0

PLUGIN_NAME="Damian Channel Strip.vst3"
BUNDLE="$CARGO_TARGET_DIR/bundled/$PLUGIN_NAME"
TARGET_DIR="$HOME/Library/Audio/Plug-Ins/VST3"
INSTALLED="$TARGET_DIR/$PLUGIN_NAME"

echo "Repo:            $ROOT"
echo "Cargo target:    $CARGO_TARGET_DIR"

echo "Building release VST3 bundle..."
cargo xtask bundle damian-channel-strip --release

if [[ ! -d "$BUNDLE" ]]; then
  echo "error: expected bundle at $BUNDLE" >&2
  exit 1
fi

BIN="$BUNDLE/Contents/MacOS/Damian Channel Strip"
if [[ ! -f "$BIN" ]]; then
  echo "error: missing plugin binary at $BIN" >&2
  exit 1
fi

echo "Built binary:"
ls -la "$BIN"
echo "  md5: $(md5 -q "$BIN")"

mkdir -p "$TARGET_DIR"

echo "Removing previous install at $INSTALLED"
rm -rf "$INSTALLED"

echo "Installing $BUNDLE -> $INSTALLED"
ditto "$BUNDLE" "$INSTALLED"
xattr -cr "$INSTALLED" 2>/dev/null || true
codesign --force --deep -s - "$INSTALLED"

INST_BIN="$INSTALLED/Contents/MacOS/Damian Channel Strip"
echo "Installed binary:"
ls -la "$INST_BIN"
echo "  md5: $(md5 -q "$INST_BIN")"

# Avoid pipefail SIGPIPE from grep -q closing strings(1) early.
UI_STRINGS_FILE="$(mktemp "${TMPDIR:-/tmp}/damian-vst3-strings.XXXXXX")"
strings "$INST_BIN" >"$UI_STRINGS_FILE"
trap 'rm -f "$UI_STRINGS_FILE"' EXIT

if grep -q "SC HPF" "$UI_STRINGS_FILE"; then
  echo "error: installed plugin still contains SC HPF UI strings" >&2
  exit 1
fi
if ! grep -q "DYNAMICS" "$UI_STRINGS_FILE"; then
  echo "error: installed plugin missing DYNAMICS UI strings" >&2
  exit 1
fi
if grep -q "COMPRESSION" "$UI_STRINGS_FILE"; then
  echo "error: installed plugin still contains old COMPRESSION UI label" >&2
  exit 1
fi
OLD_BYTES=27882720
INST_BYTES=$(wc -c < "$INST_BIN" | tr -d ' ')
if [[ "$INST_BYTES" -le "$OLD_BYTES" ]]; then
  echo "error: installed binary size ($INST_BYTES) looks like a stale build (<= $OLD_BYTES)" >&2
  exit 1
fi

echo "Installed successfully to $INSTALLED"
