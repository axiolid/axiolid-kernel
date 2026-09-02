# axiolid-linear-intersection

Certified 2D intersections for linear geometry (ADR 0036).

Allowed internal dependencies: `axiolid-core`, `axiolid-guarantees`,
`axiolid-linear`, `axiolid-predicates`. Adding any other internal dependency
breaks the declared `linear-intersection-minimal` closure and fails
`cargo xtask architecture closure check`.

## Invariants

Topology comes from certified predicates, never from comparing a determinant to
an epsilon. `Tolerance` governs residual acceptance of the returned coordinate
only; it must never be able to change a `Parallel`/`Coincident`/`Point` branch.

For unbounded lines, parallelism is a property of the DIRECTIONS. Sampling one
point from each line and comparing sides is wrong — two crossing lines can put
both samples on the same side. That bug was caught by `near_parallel_lines_still_cross`.

Results classify; they are never `Option<Point2>`. Crossing, endpoint contact,
parallel-disjoint, coincident, collinear-disjoint, and overlap are distinct
facts that topology and rule checking need.

Invalid input is a typed refusal naming the operand (`InputSide`). A zero
direction, a collapsed segment, and a non-finite coordinate are refusals, not
degenerate answers. `Disjoint` is a successful classification, never a refusal.

A certified endpoint orientation of exactly zero is stronger evidence than a
divided parameter, so segment parameters snap to exact `0.0`/`1.0` there.

## Gates

```bash
cargo test -p axiolid-linear-intersection
cargo xtask architecture closure check
```

The adversarial suite must keep asserting that every branch was generated; a
random suite that silently never produces a coincident pair proves nothing.
Operand-swap comparisons use a RELATIVE bound — the two orders evaluate
differently ordered float expressions, so bitwise equality fails spuriously.
