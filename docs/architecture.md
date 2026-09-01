# Architecture

Axiolid separates **values**, **operation contracts**, and **execution providers**. The separation is intentional: imported formats remain adapters, and hardware APIs remain replaceable providers.

## Dependency direction

```text
application / source-format adapter
              │
              ▼
  axiolid facade and leaf value crates
              │
              ▼
  axiolid-model / axiolid-kernel contracts
              │
      ┌───────┼────────┐
      ▼       ▼        ▼
 scalar    CPU context  GPU adapter seam
 oracle    providers    out-of-tree providers
```

Dependencies point downward. In particular:

- Representation crates must not depend on IFC, STEP, rendering, or GPU APIs.
- A format adapter can depend on neutral values and contracts, never a concrete execution provider.
- A provider implements an operation-specific trait; it cannot claim capability merely by being selected.
- The `axiolid` facade is optional. Leaf crates remain valid public entry points.

The full rationale is in [ADR 0009](/adr/0009-layered-geometry-dag).

## Neutral geometry DAG

`axiolid-model` stores shared geometry in an immutable, append-only graph. Nodes use typed IDs and only refer backward, making cyclic construction invalid before a compiler sees it. This supports sharing, mapped instances, reproducible diagnostics, and bounded traversal without recursive boxed shape trees.

The graph models neutral geometry intent. It does not become a source-format AST, nor does storage alone imply evaluation of every represented shape family.

## Operation contracts and providers

`axiolid-kernel` owns backend identity, policy, errors, and narrow operation contracts. Concrete implementations live outside it:

- `axiolid-reference` is readable, portable reference work used as a correctness oracle.
- `axiolid-backend-cpu` owns execution context and opt-in scheduling/dispatch support.
- `axiolid-backend-gpu` is an API-neutral adapter seam.
- `axiolid-mesh-boolean-boolmesh` is a deliberately isolated mesh Boolean provider.

This division prevents an application from accidentally pulling native or GPU dependencies into its format boundary and makes the implementation behind an operation observable.

## Tolerance and numerical work

Tolerance is explicit operation input rather than an ambient global epsilon. Numerically sensitive predicates are owned by the scalar reference layer, including filtering/escalation paths where appropriate. A faster path has to earn its complexity with a benchmark and differential evidence—not an architecture document.

See [ADR 0012](/adr/0012-scalar-reference-ownership) and [ADR 0016](/adr/0016-predicate-ownership-and-adopted-implementations).
