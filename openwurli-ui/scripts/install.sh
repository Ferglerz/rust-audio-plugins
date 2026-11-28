#!/usr/bin/env zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/../scripts/install-plugin.sh" "$ROOT/.." openwurli-ui openwurli-ui-xtask 'Electric Pianos' 'Electric Pianos' 'Electric Pianos' vst3,clap 'OpenWurli UI'
