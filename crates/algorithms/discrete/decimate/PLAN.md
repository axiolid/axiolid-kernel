# axiolid-decimate — plan

Status: edge-collapse decimation implemented with a bounded, reported
deviation. This is planning context, not standing agent instruction.

## Implemented

- `decimate` with two targets: `TriangleBudget` and `MaxDeviation`. A budget
  still honours the caller's tolerance as a deviation ceiling, so "reduce to
  N triangles" cannot licence arbitrary damage.
- Deviation is cumulative per vertex, so repeated collapses cannot drift past
  the bound one small step at a time.
- `DecimateReport` states collapses performed, refusals by cause, and the
  largest distance any vertex actually moved.
- Deterministic candidate order (length, then vertex index), verified by a
  repeated-run equality test.
- Collapse safety: the link condition (endpoints must share exactly the two
  triangles on the edge) and a normal-inversion check.

## Verified by mutation

- Removing the link condition fails `an_unsafe_collapse_is_refused_not_performed`.

## Known verification gap

The **normal-inversion branch is not covered by a mutation probe.** On every
fixture tried, the link condition rejects an unsafe collapse before the
inversion check runs, so removing the inversion check alone leaves all tests
green. The branch is written and reachable in principle, but its absence is
currently undetectable by this suite.

Closing this needs a fixture where a collapse satisfies the link condition
and *still* flips a normal — a non-convex configuration where the merged
vertex crosses a neighbouring triangle's plane while keeping exactly two
shared neighbours. That is a genuine fixture-construction problem, not a
line of code, which is why it is recorded rather than quietly left.

## Not implemented

- Isotropic remeshing and subdivision refinement (out of scope for #75).
- Sharp-feature detection. Boundary vertices are preserved implicitly by the
  link condition rather than by an explicit crease angle test; a caller
  cannot yet opt into losing them.
- Quadric error metrics. The current cost is edge length and the placement is
  the midpoint, which is simple and predictable but not optimal for a given
  triangle budget.
