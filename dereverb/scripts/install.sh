#!/usr/bin/env zsh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/../scripts/install-plugin.sh" "$ROOT/.." dereverb dereverb-xtask Dereverb Dereverb Dereverb vst3,clap
