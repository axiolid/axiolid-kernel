# axiolid-predicates instructions

Purpose: the single certified exact-arithmetic substrate (ADR 0012, ADR 0036).

Allowed internal dependencies: `axiolid-core`, `axiolid-guarantees`. Nothing
else may be added — the value of this package is that a consumer acquires
certified signs without curves, surfaces, meshes, or execution.

## Module ownership

`expansion.rs` error-free transformations; `arithmetic.rs` arbitrary-length
expansions; `orientation.rs` `orient2d`; `orient3.rs` + `orient3_dyadic.rs`
`orient3d`; `sphere.rs` `incircle`/`insphere`; `static_filter.rs` precomputed
range bounds; `scene.rs` deterministic degeneracy-controlled scene generation
for tests and benchmarks.

## Invariants

A filter may return `Uncertain`; a public predicate may not. Escalation to
exact arithmetic is the contract, not an optimisation detail. A static filter
must never certify a sign the dynamic filter would reject, and must return
`None` when an input leaves its declared coordinate range.

`Certified` is `#[non_exhaustive]`: an unrecognised variant must escalate,
never be treated as decisive.

## Gates

```bash
cargo test -p axiolid-predicates
cargo bench -p axiolid-predicates --bench predicates
```
