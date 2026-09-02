# HERMES.md — axiolid

Axiolid is a standalone pure-Rust, IFC-agnostic geometry kernel. No source-format or vendor types may enter `crates/`. The `axiolid-model` DAG is the input seam; applications select operation providers.

## Layout

- `crates/`: nested ownership tree containing 31 publishable kernel packages; `crates/facade/axiolid` is the opt-in facade.
- `tools/xtask/`: local-only architecture checker and generated-document owner.
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
scripts/gate.sh
```

The workspace has no C++ dependency path. Concrete execution providers must remain optional and require a portable scalar correctness oracle before claiming an operation trait.
