# axiolid-kernel implementation plan

Status: architecture scaffold; algorithms incomplete. This is planning context,
not standing agent instruction.

## Established

- Crate boundary and dependency direction are executable in the layering gate.
- Public operation traits compile; the mesh-boolean registry stores executable
  trait objects rather than capability metadata.

## Next implementation wave

Add a narrow batch trait only when a real implementation needs it; add an
operation-specific executable registry only when more than one provider exists.

## Exit evidence

Targeted tests, feature-isolated compile where applicable, mutation-verified
architecture/validation gates, and benchmarks before performance claims.
