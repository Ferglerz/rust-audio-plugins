#!/usr/bin/env zsh
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Bundling release VST3..."
cargo xtask bundle damian-channel-strip --release

echo "Bundle created at target/bundled/Damian Channel Strip.vst3"
