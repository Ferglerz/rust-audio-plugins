#!/usr/bin/env zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export CARGO_TARGET_DIR="$ROOT/target"
cd "$ROOT"
BUILD_ONLY=false
if [[ "${1:-}" == --build-only ]]; then
  BUILD_ONLY=true
  shift
fi
cargo build -p chordboard --features ui-preview --bin chordboard-preview

if [[ "$(uname -s)" != Darwin ]]; then
  $BUILD_ONLY && exit 0
  exec "$CARGO_TARGET_DIR/debug/chordboard-preview" "$@"
fi

# A native bundle gives Launch Services and accessibility tools a stable app
# identity. Launch with desktop GPU access, not inside an agent's sandbox.
APP="$CARGO_TARGET_DIR/preview/Chordboard Preview.app"
mkdir -p "$APP/Contents/MacOS"
cp "$CARGO_TARGET_DIR/debug/chordboard-preview" "$APP/Contents/MacOS/chordboard-preview"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>chordboard-preview</string>
  <key>CFBundleIdentifier</key><string>com.ferg.chordboard.preview</string>
  <key>CFBundleName</key><string>Chordboard Preview</string>
  <key>CFBundleDisplayName</key><string>Chordboard Preview</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$APP"
$BUILD_ONLY && exit 0
print "Opening $APP (dummy audio/MIDI backend by default)."
open "$APP" --args "$@"
