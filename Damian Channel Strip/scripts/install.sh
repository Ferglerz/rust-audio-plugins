#!/usr/bin/env zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/../scripts/install-plugin.sh" "$ROOT/.." damian-channel-strip damian-channel-strip-xtask 'Damian Channel Strip' 'Damian Channel Strip' 'Damian Channel Strip' vst3
