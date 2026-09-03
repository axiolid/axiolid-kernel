# axiolid-ray-mesh

Narrow-phase ray/triangle-mesh nearest-hit intersection (milestone v0.3, #41).

Allowed internal dependencies: `axiolid-core`, `axiolid-guarantees`,
`axiolid-mesh`, `axiolid-predicates`. `axiolid-spatial` is a DEV dependency
only: the broad phase composes with this package, it is not required by it.

## Boundary

The kernel owns the intersection and the hit record. It does not own what a ray
*means* — sampling patterns, camera rigs, entity identity, or whether a hit
counts as an obstruction stay with the caller.

## Invariants

Front/back is a certified `orient3d` sign of the ray origin against the triangle
plane, never the sign of a floating-point dot product. A ray origin exactly in
the plane reports `Coplanar` rather than picking a side.

Degenerate (zero-area) triangles and out-of-range triangle indices are typed
refusals. A degenerate triangle must never be silently skipped: that turns a
broken mesh into a plausible miss.

`t` is expressed in units of the supplied direction vector, which is not assumed
normalized. Callers wanting metres normalize first.

Coincident hits break ties by lowest triangle index, so candidate order from any
broad phase cannot change the answer. `nearest_hit_among` and `nearest_hit`
agree by construction.

`Tolerance::linear()` bounds only the parallel/edge acceptance window; it never
decides the front/back branch.

## Gates

```bash
cargo test -p axiolid-ray-mesh
cargo xtask architecture check
```

Fixtures must keep covering edge-on, vertex-on, parallel-in-plane, back-face,
behind-origin, and BVH composition. An oracle suite that never produces an
edge-on hit proves nothing about tie-breaking.
