# Contributing

Axiolid values small, proven boundaries over broad claims. A change is not complete when it compiles; it is complete when its capability, dependency direction, and failure behavior are evident.

## Local checks

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
scripts/geometry-feature-matrix.sh
scripts/probe_layering_gate.sh
```

The feature matrix protects minimal builds. The probe mutates the declared layering boundary and proves the gate fails, then verifies byte-accurate restoration.

## Design rules

- Keep Axiolid format-agnostic: source semantics belong in adapter projects.
- Do not add a concrete backend dependency to a representation or format boundary.
- Treat scalar paths as correctness oracles; benchmark and differentially test a faster path before claiming a performance win.
- Keep operation capability tied to an executable provider implementation.
- Record irreversible architecture choices under [`docs/adr/`](../adr/README.md).
- Update a crate `PLAN.md` only with concrete next work, not aspirational coverage claims.

## Documentation

The site is VitePress and deploys from `main` through GitHub Pages. Preview it locally:

```bash
npm --prefix docs ci
npm --prefix docs run docs:dev
```
