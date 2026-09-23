#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

cargo check -p pleasant-dsp
cargo check -p pleasant-curves
cargo check -p pleasant-eq
cargo check -p pleasant-dynamics
cargo check -p pleasant-ui
cargo check -p damian-channel-strip
cargo check -p flattery
cargo check -p tape_stop

cargo test -p pleasant-dsp --quiet
cargo test -p pleasant-curves --quiet
cargo test -p pleasant-eq --quiet
cargo test -p pleasant-dynamics --quiet
cargo test -p damian-channel-strip --quiet
cargo test -p flattery --quiet
cargo test -p tape_stop --quiet

cargo check -p pleasant-dsp-headless
cargo check -p pleasant-curves-headless
cargo check -p pleasant-eq-headless

cargo check --manifest-path scd-rust/Cargo.toml -p scd-plugin --offline
cargo test --manifest-path scd-rust/Cargo.toml -p scd-plugin --lib --quiet
