# axiolid-field-ops instructions

Purpose: frame-neutral, deterministic algorithms over sampled layered fields.

Allowed internal dependencies: `axiolid-core`, `axiolid-field`. Follow parent
`../AGENTS.md`. Do not read `PLAN.md` unless assigned implementation or roadmap
work.

## Module ownership

Field values, configuration, evidence, and invariant-preserving constructors
belong to `axiolid-field`. This crate owns `sample.rs` scalar CPU triangle
coverage; `morphology.rs` masks, metric
dilation/erosion, connected components; `clearance.rs` gap queries;
`navigate.rs` geometry-only traversal behind the `navigation` feature.

## Invariants

A cell holds **two separate channels**. `SurfaceHit`s are zero-thickness
crossings with a facing sign; occupancy is a set of strictly positive,
strictly disjoint intervals. Triangle coverage emits surface hits **only** — a
triangle has no thickness and must never be reported as occupied volume.
Occupancy is derived in a separate step from alternating enter/exit crossings
and requires a closed shell; an unbalanced sequence is
`UnbalancedCrossings`, and a tolerance-collapsed span is `DegenerateOccupancy`.
Neither is repaired silently.

No world axis is assumed. The caller supplies a validated right-handed
`Frame3`; local `z` is the layering axis. Bounds, cell size, and radii are in
local units. A caller wanting Z-up passes the identity frame explicitly.

There is no built-in resource cap. `FieldResourceBudget` is caller-owned, and
exhaustion is `CellBudgetExceeded` or `SampleBudgetExceeded` — never a silent
truncation. Tolerance is explicit per field; there is no global epsilon.

Determinism is a contract: cells are row-major, surface hits sort by `w` then
facing, components are labelled in scan order with the lowest reachable index,
and route ties break by cost then node index. Repeated runs are bit-identical.

Sampling never invents data. Parallel rays, degenerate triangles, out-of-bounds
crossings, edge/vertex contacts, and merged coincident hits from shared facet
edges are all counted in `SamplingEvidence` rather than dropped quietly.

## Navigation boundary

`navigate.rs` may report `route exists`, `no route under this envelope`, and
`clearance = X` with geometric rejection evidence. It must never name a domain
verdict: no accessibility, ADA, wheelchair, egress, escape-route, code
compliance, or vendor rule vocabulary in any type, field, variant, or doc.
Envelope values are geometry (`agent_radius`, `agent_height`, `max_step`,
`max_slope`), not policy. The feature is opt-in and stays that way until a
second independent consumer needs the same neutral contract.

## Gates

```bash
cargo test -p axiolid-field-ops --all-features
python3 scripts/probe_field_gate.py
```

Mutation probes must kill every defect before the suite is trusted. Probe
anchors are matched against rustfmt-normalised sources — re-read the formatted
file after `cargo fmt`, or anchors silently miss and probes report as leaked.
