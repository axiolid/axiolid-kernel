#!/usr/bin/env bash
# Full gate for Axiolid. Trusts exit codes, not parsed output.
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."
gate_out="$(mktemp "${TMPDIR:-/tmp}/axiolid-gate.XXXXXX")" || exit 1
trap 'rm -f "$gate_out"' EXIT
fail=0
step() { local name="$1"; shift; printf '%-46s' "$name"; if "$@" >"$gate_out" 2>&1; then echo ok; else echo "FAIL (exit $?)"; tail -25 "$gate_out" | sed 's/^/    /'; fail=1; fi; }
step "fmt --check" cargo fmt --all -- --check
step "build --workspace" cargo build --workspace
step "test --workspace" cargo test --workspace
step "test --all-features" cargo test --workspace --all-features
step "clippy" cargo clippy --workspace --all-targets -- -D warnings
step "clippy --all-features" cargo clippy --workspace --all-targets --all-features -- -D warnings
step "doc" env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
step "feature matrix" scripts/geometry-feature-matrix.sh
for c in axiolid-core axiolid-mesh axiolid-profile axiolid-curve axiolid-surface axiolid-topology axiolid-model axiolid-primitive axiolid-sweep axiolid-tessellate axiolid-spatial axiolid-measure axiolid-heal axiolid-kernel axiolid-backend-cpu axiolid-backend-gpu axiolid; do step "isolated build -p $c" cargo build -p "$c"; done
echo
[ "$fail" -eq 0 ] && echo "GATE PASSED" || echo "GATE FAILED"
exit "$fail"
