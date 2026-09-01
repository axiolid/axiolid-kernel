# 0016 — Predicate ownership alongside adopted implementations

- **Status:** Accepted
- **Date:** 2026-08-19
- **Deciders:** Friedrich, axiolid
- **Relates to:** [0003](0003-pure-rust-mesh-boolean.md), [0012](0012-scalar-reference-ownership.md), [0014](0014-adopt-boolmesh-mesh-boolean.md), [0015](0015-adopt-earcut-polygon-triangulation.md)

## Context

`axiolid-scalar` now owns a certified predicate suite: `orient2d`, `orient3d`,
`incircle`, `insphere`, plus static filters. Their filtered cascades escalate
to exact arithmetic, so they return a *proven* sign rather than a plausible
one. `orient3d` normally uses expansion arithmetic; finite exponent ranges
that would overflow or underflow those expansions use a fixed-size exact
dyadic accumulator instead.

But the mesh boolean is `boolmesh` (ADR 0014) and polygon triangulation is
`earcut` (ADR 0015). Both carry their own predicates. So the obvious question:
if we do not use ours inside the algorithms that matter most, why own them?

The naive answers are both wrong:

- *"Replace theirs with ours."* `boolmesh` is MPL-2.0. Substituting its
  predicates means modifying its files, which triggers file-level copyleft on
  the result and forks us off upstream fixes. It also discards the one thing
  adoption bought: someone else maintaining a working boolean.
- *"Then do not build ours."* Predicates are not only for booleans. Anything we
  implement ourselves needs them, and without an independent implementation we
  have no way to audit an adopted one.

## Decision

Our predicates serve three roles, none of which require displacing an adopted
implementation.

**1. They are the substrate for algorithms we own.** Ear-clipping orientation
in `axiolid-scalar::polygon` already calls `orient2d`. Any future spatial index,
sweep, or healing pass built here uses them by default. This is the primary
role and it needs no negotiation with anyone's licence.

**2. They are the audit oracle for adopted implementations.** ADR 0012 makes
`axiolid-scalar` the correctness reference. An adopted crate is *verified, not
trusted*: `axiolid-compile/tests/oracle.rs` already checks `earcut` against our
certified triangulator on every hole-free polygon. The predicate suite extends
that pattern to geometric decisions — we can assert that an adopted result is
consistent with signs we can prove, without touching its source.

**3. They are the exit option.** If an adopted crate is abandoned, changes
licence, or is found wrong on our corpus, the replacement cost is bounded
because the hard part — certified arithmetic — is already ours and tested.
Owning the predicates is what makes ADR 0014's "the seam makes that swap cheap"
true rather than aspirational.

What we explicitly do **not** do: patch `boolmesh` to use our predicates, or
vendor it to make that possible. The cost is real (its predicate quality is
outside our control) and it is accepted, with the volume-conservation and
manifold gates as the detection mechanism.

## Measured behaviour

Historical benchmarks (`cargo bench -p axiolid-scalar`, Xeon w7-3565X,
200k samples/tier) are retained below as provenance. They predate the 2026-09-01
correction that made `orient3d` exact escalation preserve coordinate-difference
tails, so they are not a current post-fix performance claim. Throughput and
escalation rate were reported together because neither number is interpretable
alone.

| predicate | 0% | 0.01% | 1% | 10% |
| --- | --- | --- | --- | --- |
| `orient2d` | 93.3 M/s | 93.5 M/s | 98.0 M/s | 89.9 M/s |
| `orient3d` | 72.5 M/s | 75.4 M/s | 66.6 M/s | 32.1 M/s |

Escalation tracks the injected degeneracy rate to four decimal places
(0.0000%, 0.0100%, 1.0000%, 10.0000%), which is the gate asserted in
`tests/degeneracy.rs`.

The answer to "does robustness collapse on bad data": **no, it degrades
proportionally.** `orient2d` is essentially flat — its exact path is cheap
enough to disappear into measurement noise even at 10% degeneracy. `orient3d`
costs more at high degeneracy, because its exact path builds expansion cofactors
and, when coordinate subtraction has a nonzero tail, multiplies complete
difference expansions. Finite inputs outside the expansion evaluator's checked
no-overflow/no-underflow envelope use a bounded 100-limb stack accumulator over
exact binary64 dyadics; this avoids intermediate overflow and underflow without
adding heap allocation to that rare fallback. That is a real cost and it is
bounded by the finite expansion sizes for this fixed-degree predicate: a 10%
exactly-coplanar rate is far beyond authored building models, and the price of
the alternative (a wrong sign) is a corrupt mesh.

The static filter is 1.09x faster than the dynamic one on clean 2D data. That
is a smaller win than the theory suggests, because the dynamic permanent is
already cheap for `orient2d`. It is kept because it declines 0% of in-range
clean inputs, and because the margin grows with the predicate's degree.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Fork `boolmesh` and swap in our predicates | MPL-2.0 file-level copyleft on modifications, plus permanent divergence from upstream. Buys predicate control at the cost of maintaining a boolean kernel — exactly what ADR 0014 decided not to do. |
| Skip our own predicates entirely | Leaves every algorithm we own without a certified foundation, removes the audit oracle, and makes an adopted-crate failure unrecoverable. |
| Use a third-party predicate crate (`robust`) | Reasonable, and it would have saved effort. Rejected because ADR 0012 requires the scalar reference to be *ours* to be a trustworthy oracle: auditing one adopted crate with another does not establish independence. |

## Consequences

**Positive**

- Certified signs available to every algorithm in the workspace.
- Adopted crates can be audited rather than trusted, extending the ADR 0015
  differential-oracle pattern to geometric decisions.
- Replacement cost for an adopted crate is bounded.

**Negative / costs**

- Duplicate predicate implementations exist in the dependency graph (ours and
  `boolmesh`'s). This is accepted duplication, not an oversight.
- The exact paths allocate. Measured above; bounded by the escalation rate,
  which is gated.
- `incircle` and `insphere` currently have **no production consumer**. They are
  proven correct against independent oracles but not yet load-bearing. Recorded
  plainly rather than presented as delivered capability.

**Follow-ups**

- Wire `orient3d` into a manifold-orientation check so `axiolid-compile` validates
  its own output with certified signs rather than a raw determinant.
- Add a differential harness comparing `boolmesh` boolean results against
  certified-sign expectations on the fixture corpus, closing the audit loop for
  the adopted boolean specifically.

## Relation to existing code

- `crates/axiolid-scalar/src/{orientation,orient3,sphere,static_filter}.rs`
- `crates/axiolid-scalar/src/scene.rs` — degeneracy-controlled scenes.
- `crates/axiolid-scalar/benches/predicates.rs` — the measurements above.
- `crates/axiolid-scalar/tests/degeneracy.rs` — escalation-rate gates.
