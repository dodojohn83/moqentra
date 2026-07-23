#!/usr/bin/env bash
# R1-QA-001: Rust quality gate.
# Runs fmt, clippy, build and tests with logs written to target/test-reports/.
set -euo pipefail

export CARGO_TERM_COLOR=never
export RUST_BACKTRACE=1

REPORTS_DIR="${REPORTS_DIR:-target/test-reports}"
mkdir -p "$REPORTS_DIR"

echo "Running cargo fmt..."
cargo fmt --all -- --check > "$REPORTS_DIR/fmt.log" 2>&1

echo "Running cargo clippy..."
cargo clippy --workspace --all-targets -- -D warnings \
    > "$REPORTS_DIR/clippy.log" 2>&1

echo "Running cargo build..."
cargo build --workspace --all-targets \
    > "$REPORTS_DIR/build.log" 2>&1

echo "Running tests..."
if command -v cargo-nextest >/dev/null 2>&1 || cargo install --list | grep -q '^cargo-nextest'; then
    cargo nextest run --workspace --failure-output final-flatten \
        > "$REPORTS_DIR/nextest.log" 2>&1
else
    cargo test --workspace -- --nocapture \
        | tee "$REPORTS_DIR/cargo-test.log" 2>&1
fi

echo "Rust QA gate passed. Reports are in $REPORTS_DIR."
