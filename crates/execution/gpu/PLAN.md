# axiolid-backend-gpu implementation plan

Status: architecture scaffold; algorithms incomplete. This is planning context,
not standing agent instruction.

## Established

- Crate boundary and dependency direction are executable in the layering gate.
- Public data/contracts compile. Behavior remains scaffold unless a test names it.
- The generic adapter validates device and precision policy, graph-owned roots,
  and one-result-per-root cardinality before accepting executor output.

## Next implementation wave

Add a separately feature-gated wgpu graph compiler only with real batched
compute kernels and CPU differential tests. Add other GPU operation executors
as separate traits/adapters, not methods on one god backend.

## Exit evidence

Targeted tests, feature-isolated compile where applicable, mutation-verified
architecture/validation gates, and benchmarks before performance claims.
