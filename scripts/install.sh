#!/usr/bin/env zsh
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building release VST3 bundle..."
cargo xtask bundle damian-channel-strip --release

TARGET_DIR="$HOME/Library/Audio/Plug-Ins/VST3"
mkdir -p "$TARGET_DIR"

echo "Installing to $TARGET_DIR..."
cp -R "target/bundled/Damian Channel Strip.vst3" "$TARGET_DIR/"

echo "Installed successfully to $TARGET_DIR/Damian Channel Strip.vst3"
