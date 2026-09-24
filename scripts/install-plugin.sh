#!/usr/bin/env zsh
set -euo pipefail

if [[ $# -ne 7 ]]; then
  echo "usage: install-plugin.sh WORKSPACE CRATE XTASK_PACKAGE BUNDLE_NAME INSTALL_NAME BINARY_NAME FORMATS" >&2
  exit 2
fi

WORKSPACE="$(cd "$1" && pwd)"
CRATE="$2"
XTASK_PACKAGE="$3"
BUNDLE_NAME="$4"
INSTALL_NAME="$5"
BINARY_NAME="$6"
FORMATS="$7"

cd "$WORKSPACE"
export CARGO_TARGET_DIR="$WORKSPACE/target"
BUNDLED_DIR="$CARGO_TARGET_DIR/bundled"

# Cargo normally reuses correct artifacts, but installation always forces this
# plugin's release binary and bundles to come from the current checkout.
cargo clean -p "$CRATE" --release
for format in ${(s:,:)FORMATS}; do
  rm -rf "$BUNDLED_DIR/$BUNDLE_NAME.$format"
done

echo "Building $CRATE from $WORKSPACE..."
cargo run -p "$XTASK_PACKAGE" --release -- bundle "$CRATE" --release

# Validate every source before touching an installed plugin.
for format in ${(s:,:)FORMATS}; do
  source_binary="$BUNDLED_DIR/$BUNDLE_NAME.$format/Contents/MacOS/$BINARY_NAME"
  if [[ ! -f "$source_binary" ]]; then
    echo "Missing build output: $source_binary" >&2
    exit 1
  fi
done

for format in ${(s:,:)FORMATS}; do
  case "$format" in
    vst3) destination_dir="$HOME/Library/Audio/Plug-Ins/VST3" ;;
    clap) destination_dir="$HOME/Library/Audio/Plug-Ins/CLAP" ;;
    *) echo "Unsupported plugin format: $format" >&2; exit 2 ;;
  esac
  mkdir -p "$destination_dir"
  source_bundle="$BUNDLED_DIR/$BUNDLE_NAME.$format"
  installed_bundle="$destination_dir/$INSTALL_NAME.$format"

  # A second bundle name can make the DAW keep loading an older copy.
  rm -rf "$installed_bundle"
  if [[ "$BUNDLE_NAME" != "$INSTALL_NAME" ]]; then
    rm -rf "$destination_dir/$BUNDLE_NAME.$format"
  fi
  if [[ "$CRATE" != "$INSTALL_NAME" && "$CRATE" != "$BUNDLE_NAME" ]]; then
    rm -rf "$destination_dir/$CRATE.$format"
  fi
  cp -R "$source_bundle" "$installed_bundle"
  cmp "$source_bundle/Contents/MacOS/$BINARY_NAME" "$installed_bundle/Contents/MacOS/$BINARY_NAME"
  xattr -cr "$installed_bundle" 2>/dev/null || true
  codesign --force --deep -s - "$installed_bundle"
  codesign --verify --deep "$installed_bundle"
  echo "Installed $installed_bundle"
done
