# axiolid-compile plan

## Done
- Profile flattening: rectangle, circle, ellipse, hollow variants, contours,
  and `Derived` (2D placement) which every real IFC profile uses.
- `earcut` triangulation with holes (ADR 0015); `axiolid-reference` audits it.
- Linear extrusion with caps and sides; edge-parity verified.
- `ScalarCompiler`: iterative post-order walk, memoised, boolean dispatch.

## Invariants
- Outer rings CCW, holes CW. Mirrored placements are re-oriented, never
  passed through: a negative-determinant transform silently inverts a solid.
- Volume alone cannot gate winding. A cap in the z=0 plane contributes
  nothing to the divergence integral, so a flipped cap is invisible to it.
  Directed-edge parity is the winding-sensitive gate.
- Unsupported families return `Unsupported` naming the capability needed.

## Next
- Revolution (seam handling), swept disk, B-rep, tessellated face sets.
- `subtract_many` batch override once a workload justifies it.
