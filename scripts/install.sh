#!/usr/bin/env zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/../scripts/install-plugin.sh" "$ROOT/.." openwurli-ui openwurli-ui-xtask 'OpenWurli UI' 'OpenWurli UI' 'OpenWurli UI' vst3,clap
