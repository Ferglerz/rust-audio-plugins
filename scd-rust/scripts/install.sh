#!/usr/bin/env zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/../scripts/install-plugin.sh" "$ROOT" scd-plugin xtask 'SoundChef Drums' 'SoundChef Drums' 'SoundChef Drums' vst3,clap
