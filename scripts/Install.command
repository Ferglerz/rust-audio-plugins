#!/bin/bash
set -euo pipefail

# Clears macOS quarantine on this folder. Does not copy plugins.
# Recipients who do not want to approve a script: skip this file and use Read Me.txt.

HERE="$(cd "$(dirname "$0")" && pwd)"
DEST="${HOME}/Library/Audio/Plug-Ins/VST3"

pause() {
  echo
  echo "Press Return to close."
  read -r _ || true
}

echo "This only strips quarantine. It does not move plugins."
echo
echo "Folder:"
echo "  $HERE"
echo
echo "After this, copy the .vst3 bundles into:"
echo "  $DEST"
echo

case "$HERE" in
  *AppTranslocation*)
    echo "error: macOS ran this script from a temporary Gatekeeper copy," >&2
    echo "so it cannot see the plugins sitting next to it." >&2
    echo >&2
    echo "Close this window. Open Terminal and paste:" >&2
    echo >&2
    echo "  xattr -cr " >&2
    echo >&2
    echo "Then drag the unzipped plugin folder onto the Terminal window and press Return." >&2
    pause
    exit 1
    ;;
esac

echo "Clearing com.apple.quarantine on this folder..."
if ! /usr/bin/xattr -cr "$HERE"; then
  echo "error: could not clear attributes. Grant Terminal access to this folder in" >&2
  echo "System Settings → Privacy & Security → Files and Folders, then run again." >&2
  pause
  exit 1
fi

echo "Done. Drag the .vst3 folders into the VST3 directory."
mkdir -p "$DEST"
open "$HERE"
open "$DEST"
pause
