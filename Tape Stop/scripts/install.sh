#!/usr/bin/env zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/../scripts/install-plugin.sh" "$ROOT/.." tape_stop tape-stop-xtask 'Tape Stop' 'Tape Stop' 'Tape Stop' vst3
