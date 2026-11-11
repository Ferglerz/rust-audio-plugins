#!/usr/bin/env zsh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/../scripts/install-plugin.sh" "$ROOT/.." automixer automixer-xtask Automixer Automixer Automixer vst3,clap
