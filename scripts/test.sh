#!/usr/bin/env zsh
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Running tests..."
cargo test

echo "Running clippy..."
cargo clippy -- -D warnings

echo "Checking formatting..."
cargo fmt --check

echo "All checks passed!"
