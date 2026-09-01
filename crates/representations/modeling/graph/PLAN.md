# axiolid-model implementation plan

Status: architecture scaffold; algorithms incomplete. This is planning context,
not standing agent instruction.

## Established

- Crate boundary and dependency direction are executable in the layering gate.
- Public data/contracts compile. Behavior remains scaffold unless a test names it.
- Node handles carry a graph-owner brand. Insertion rejects foreign, forward, and
  semantically invalid reference families before an immutable graph can exist.

## Next implementation wave

Add graph visitors, budgets, provenance side tables, and complete compiler coverage.

## Exit evidence

Targeted tests, feature-isolated compile where applicable, mutation-verified
architecture/validation gates, and benchmarks before performance claims.
