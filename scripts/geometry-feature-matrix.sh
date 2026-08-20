#!/usr/bin/env bash
# Feature-isolated geometry build matrix. Keep in sync with axiolid/Cargo.toml.
set -euo pipefail

cd "$(dirname "$0")/.."

step() {
    printf '%-52s' "$1"
    shift
    "$@" >/dev/null
    printf 'ok\n'
}

step "axiolid facade: core only" cargo check -q -p axiolid --no-default-features

features=(
    mesh profiles curves surfaces topology primitives model
    sweeps tessellation spatial measure heal
    kernel mesh-boolean graph-compile
    cpu parallel simd gpu
    discrete parametric advanced full
)
for feature in "${features[@]}"; do
    step "axiolid facade feature: ${feature}" \
        cargo check -q -p axiolid --no-default-features --features "$feature"
done

step "axiolid facade: defaults" cargo test -q -p axiolid
step "axiolid facade: all features" cargo test -q -p axiolid --all-features

step "kernel contract: identity only" \
    cargo check -q -p axiolid-kernel --no-default-features
step "kernel contract: mesh boolean" \
    cargo test -q -p axiolid-kernel --no-default-features --features mesh-boolean
step "kernel contract: graph model" \
    cargo check -q -p axiolid-kernel --no-default-features --features model
step "kernel contract: all" cargo test -q -p axiolid-kernel --all-features

step "CPU context: portable" \
    cargo check -q -p axiolid-backend-cpu --no-default-features
step "CPU context: SIMD" \
    cargo check -q -p axiolid-backend-cpu --no-default-features --features simd
step "CPU context: parallel" \
    cargo check -q -p axiolid-backend-cpu --no-default-features --features parallel
step "CPU context: SIMD + parallel" \
    cargo test -q -p axiolid-backend-cpu --all-features
step "GPU adapter contract" cargo test -q -p axiolid-backend-gpu

if rustup target list --installed | grep -qx 'aarch64-linux-android'; then
    step "CPU context: AArch64 compile" \
        cargo check -q -p axiolid-backend-cpu --target aarch64-linux-android \
        --no-default-features --features simd
else
    printf '%-52s%s\n' "CPU context: AArch64 compile" "skip (target not installed)"
fi
