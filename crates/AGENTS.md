# Crates

This directory is physically organized by architectural ownership. Folders communicate ownership; Cargo packages remain the actual dependency/trust/compilation boundaries.

## Direct children

- `foundation/` — unique dependency root (`axiolid-core`).
- `representations/` — portable analytic, region, topology, B-rep, mesh, sampled-field, and authored-graph values.
- `contracts/` — guarantees, common vocabulary, mesh admissibility, and operation-specific portable schemas.
- `algorithms/` — format-neutral reference, parametric, construction, planar, query, sampled, and repair implementations.
- `providers/` — concrete optional operation providers.
- `execution/` — provider dispatch, graph execution, and CPU/GPU contexts/adapters.
- `facade/` — feature-gated public `axiolid` package.

Read the nested `AGENTS.md` before editing a child.

## Dependency rules

`cargo xtask architecture check` validates package metadata, explicit members, exact internal dependency allowlists, production/build role direction, nested placement, source-format neutrality, placeholders, unsafe policy, and generated-doc freshness.

Production direction is:

```text
foundation <- representations <- contracts <- algorithms/providers <- execution <- facade
```

This is a role DAG, not a license to depend on every earlier layer. Exact package edges remain allowlisted. Contracts may depend on representation values required by typed schemas, but never providers or dispatch. Algorithms do not select execution policy. Explicit dev-only upward edges are limited to integration/conformance tests.

## Required gates

From repository root:

```bash
cargo xtask architecture check
scripts/probe_layering_gate.sh
scripts/geometry-feature-matrix.sh
cargo test --workspace --all-features
```

Run `cargo xtask architecture docs` after metadata/package changes, then rerun the checker.
