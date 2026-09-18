#!/usr/bin/env zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PLUGIN_NAME="Damian Channel Strip.vst3"
INSTALLED="$HOME/Library/Audio/Plug-Ins/VST3/$PLUGIN_NAME"

echo "=== purging Damian Channel Strip build + install caches ==="

# Repo target
if [[ -d "$ROOT/target" ]]; then
  echo "removing $ROOT/target"
  rm -rf "$ROOT/target"
fi

# pleasant-ui target (path dependency)
PUI="$ROOT/../pleasant-ui/target"
if [[ -d "$PUI" ]]; then
  echo "removing $PUI"
  rm -rf "$PUI"
fi

# Installed VST3
if [[ -e "$INSTALLED" ]]; then
  echo "removing $INSTALLED"
  rm -rf "$INSTALLED"
fi

# Cursor sandbox cargo targets that may contain this plugin
while IFS= read -r bundle; do
  cache_root="${bundle%/bundled/$PLUGIN_NAME}"
  echo "removing sandbox cache $cache_root"
  rm -rf "$cache_root"
done < <(find /var/folders -path "*/cursor-sandbox-cache/*/cargo-target/bundled/$PLUGIN_NAME" 2>/dev/null || true)

# macOS Audio Unit validation caches (Logic and some hosts)
for cache in \
  "$HOME/Library/Caches/AudioUnitCache" \
  "$HOME/Library/Caches/com.apple.audiounits.cache"; do
  if [[ -e "$cache" ]]; then
    echo "removing $cache"
    rm -rf "$cache"
  fi
done

echo "purge complete"
