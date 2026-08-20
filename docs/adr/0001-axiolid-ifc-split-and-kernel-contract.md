# 0001 — Split IFC semantics from Axiolid, the external geometry kernel

- **Status:** Accepted
- **Date:** 2026-08-18
- **Deciders:** GeneralPawz, Hermes
- **Supersedes:** —

## Context

IFC geometry is a source-adapter concern, not a property of a reusable geometry kernel. A kernel that knows IFC entities cannot serve other importers; an IFC codec that selects a backend forces every consumer to inherit that backend.

The project also rejects the common failure mode where an ordinary geometry dependency introduces a heavy native C++ toolchain. Axiolid must remain pure Rust and source-format neutral.

## Decision

Axiolid is an independent geometry workspace. Source-format adapters translate their data into Axiolid values and neutral geometry graphs at the boundary.

- Axiolid crates do not expose IFC, STEP, renderer, or GPU API vocabulary.
- The kernel defines operation contracts separately from CPU, GPU, and third-party providers.
- Adapters depend on data and contracts only; applications choose and compose concrete providers.

```text
source-format adapter → Axiolid values / graph → operation contract → provider
        IFC/STEP            format-neutral          stable seam       scalar/CPU/GPU
```

## Alternatives considered

| Option | Why not |
| --- | --- |
| Build geometry into the IFC layer | Makes kernel concepts IFC-shaped and prevents reuse by other formats. |
| Depend on a single geometry facade | Couples the adapter to one implementation and obscures provider replacement. |
| Pick a provider with compile-time features | Produces hardware-specific builds and prevents simultaneous differential testing. |
| Put a backend parameter on every IFC type | Spreads an execution choice through unrelated schema and codec APIs. |

## Consequences

**Positive**

- Format adapters stay small and format-specific.
- A mesh-only consumer does not pay for topology, GPU, or source-format machinery.
- Scalar, optimized, and accelerator providers can be tested against the same neutral requests.

**Negative / costs**

- The integration boundary must be designed and preserved deliberately.
- Provider capabilities need explicit contracts rather than implicit assumptions.

## Relation to existing code

- `crates/axiolid-core` owns neutral scalar values, transforms, bounds, and tolerance policy.
- `crates/axiolid-model` owns the immutable geometry graph.
- `crates/axiolid-kernel` owns operation contracts; providers are separate crates behind opt-in features.
- The Nehirde IFC geometry adapter is a consumer of this boundary, not an Axiolid dependency.
