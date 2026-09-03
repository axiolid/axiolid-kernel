#!/usr/bin/env bash
# Keep the public downstream contract synchronized with executable Rust definitions.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."
actual="$(mktemp "${TMPDIR:-/tmp}/axiolid-integration-contract.XXXXXX")"
trap 'rm -f "$actual"' EXIT
cargo run -q -p axiolid-contracts --example downstream_profiles >"$actual"
diff -u docs/architecture/downstream-integration.md "$actual"
echo "downstream integration contract is current"
