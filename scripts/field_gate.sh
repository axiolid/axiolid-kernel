#!/usr/bin/env bash
# Full verification gate for the axiolid-field slice.
set -euo pipefail

unset CARGO_NET_OFFLINE CARGO_HOME GIT_DIR GIT_WORK_TREE 2>/dev/null || true
export RUSTUP_TOOLCHAIN=1.88.0

cd /mnt/backup/build-cache/axiolid-solibri-spatial

echo "=== 1. fmt ==="
cargo fmt --all -- --check
echo "fmt OK"

echo "=== 2. field contract suites ==="
cargo test -p axiolid-field --all-features

echo "=== 3. layering gate ==="
cargo test -p axiolid-core --test layering

echo "=== 4. workspace tests, all features ==="
cargo test --workspace --all-features

echo "=== 5. clippy ==="
cargo clippy --workspace --all-targets --all-features -- -D warnings
echo "clippy OK"

echo "=== 6. rustdoc ==="
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
echo "doc OK"

echo "=== 7. feature isolation ==="
cargo check -p axiolid-field --no-default-features
cargo check -p axiolid-field --features navigation
cargo check -p axiolid --features field
cargo check -p axiolid --features field-navigation
echo "feature isolation OK"

echo "=== 8. whitespace ==="
git diff --check
echo "whitespace OK"

echo "=== GATE COMPLETE ==="
