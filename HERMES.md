# HERMES.md — axiolid

Axiolid is a standalone pure-Rust, IFC-agnostic geometry kernel. No source-format or vendor types may enter `crates/`. The `axiolid-model` DAG is the input seam; applications select operation providers.

## Layout

- `crates/`: independently publishable kernel crates; `axiolid` is the opt-in facade.
- `docs/adr/`: durable architecture decisions.
- `docs/research/`: prior-art evidence.
- `scripts/`: feature and mutation-verified architecture gates.

## Commands

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
scripts/geometry-feature-matrix.sh
scripts/probe_layering_gate.sh
scripts/gate.sh
```

The workspace has no C++ dependency path. Concrete execution providers must remain optional and require a portable scalar correctness oracle before claiming an operation trait.
