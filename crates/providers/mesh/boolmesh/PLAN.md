# axiolid-mesh-boolean-boolmesh plan

Owner: geometry
Depends on: axiolid-mesh-boolean-contract, axiolid-mesh-contracts, axiolid-contracts, axiolid-mesh, axiolid-core

## Done

- [x] TriMesh <-> Manifold conversion with an orientation gate on input.
- [x] `MeshBoolean` for union/intersection/difference.
- [x] Volume-conservation and winding gates; fixture issue_2019 regression.
- [x] Registry integration, including budget refusal.

## Next

- [ ] Override `subtract_many` to union disjoint cutters before subtracting, and
      prove it beats the sequential baseline recorded in ADR 0014
      (n=16: 6.95 ms, n=64: 48.68 ms). If it does not beat it, do not keep it.
- [x] Fixture issue_1155 (near-degenerate halfspace) as a regression here. The
      half-space is still bounded in the test; moving that bounding into
      axiolid-model remains open.
- [x] Differential test against certified `axiolid-predicates` (`orient3d`).
      Convexity, inside/outside and winding are re-decided exactly where those
      invariants hold; non-convex results stay covered by the conservation and
      structural gates instead, because "wound away from one interior point" is
      only true for convex solids.
