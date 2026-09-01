# axiolid-reference plan

## Done
- Error-free transformations (`two_sum`, `two_diff`, `two_product`).
- `orient2d`: filtered cascade escalating to exact expansion arithmetic.

## In progress: the predicate suite
- [ ] Arbitrary-length expansion arithmetic (the foundation all of these share).
- [ ] `orient3d`  — is a point above/on/below a plane.
- [ ] `incircle`  — is a point inside/on/outside a circumcircle.
- [ ] `insphere`  — is a point inside/on/outside a circumsphere.
- [ ] Static filters: precomputed bounds from a coordinate magnitude limit,
      skipping the per-call permanent computation.
- [ ] Degeneracy benchmark harness: throughput AND escalation rate at
      0%, 0.01%, 1%, 10% degenerate inputs.

## Gates
- Differential vs an independent exact oracle (i128 rational, integer inputs
  bounded so the determinant cannot overflow).
- Measured escalation rate per degeneracy tier, asserted to stay in band.
- Mutation probes on every filter bound and every exact path.

## Relationship to adopted predicates
`boolmesh` carries its own predicates and is MPL-2.0, so replacing them means
forking. See ADR 0016: ours serve our own algorithms and act as an independent
audit oracle for adopted ones, rather than trying to displace them.
