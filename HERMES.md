# HERMES.md — axiolid

Axiolid is a standalone pure-Rust, IFC-agnostic geometry kernel. No source-format or vendor types may enter `crates/`. The `axiolid-model` DAG is the input seam; applications select operation providers.

## Layout

- `crates/`: nested ownership tree containing publishable kernel packages; `crates/facade/axiolid` is the opt-in Rust facade and `crates/facade/axiolid-capi` is the sole unsafe C ABI boundary.
- `tools/xtask/`: local-only architecture checker and generated-document owner.
- `native/`: source-build and installed-package CMake integration; immutable source pins only.
- `docs/adr/`: durable architecture decisions; ADR 0035 owns the current package topology.
- `docs/architecture/`: current and generated crate/dependency maps.
- `docs/research/`: prior-art evidence.
- `scripts/`: feature, release, conformance, and mutation-verified architecture gates.

## Commands

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo xtask architecture check
bash scripts/probe_layering_gate.sh
bash scripts/field_gate.sh
bash scripts/geometry-feature-matrix.sh
scripts/check-capi.sh
scripts/check-native-packaging.sh
scripts/gate.sh
```

The workspace has no C++ dependency path. `axiolid-capi` is the sole audited unsafe Rust boundary and must deny unsafe operations in unsafe functions; every other facade, contract, representation, and foundation crate forbids unsafe code. Concrete execution providers must remain optional and require a portable scalar correctness oracle before claiming an operation trait.
