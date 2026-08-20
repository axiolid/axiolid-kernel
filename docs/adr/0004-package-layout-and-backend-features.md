# 0004 — Package-group layout; backends as features of `axiolid-kernel`

- **Status:** Superseded by [0009](0009-layered-geometry-dag.md)
- **Date:** 2026-08-18
- **Deciders:** GeneralPawz, Hermes
- **Supersedes:** amends the *layout* in 0001 and the *packaging* in 0002. The
  reasoning in both stands unchanged; only where code lives changed.

## Context

0001 established a two-group split (`axiolid/`, `ifc/`) and 0002 gave each hardware
backend its own crate (`axiolid-cpu`, `axiolid-simd`, `axiolid-gpu`, `axiolid-dispatch`).
Two forces then applied:

1. The workspace needs room for domains beyond the IFC core — openBIM standards
   (IDS, BCF), bindings (Python, wasm), and applications. Flat top-level dirs
   for each would put `bcf/` beside `axiolid/` as though they were peers, which
   they are not: `bcf` is a consumer of the IFC layer.
2. Four crates that each contain one backend impose real cost — four manifests,
   four version bumps, four entries in the dependency table — for no isolation
   benefit, since they share the same traits and the same data types.

## Decision

**Layout: group by role under `packages/`, with `bindings/` and `apps/` as
peers.**

```
crates/{axiolid-core,axiolid-mesh,axiolid-brep,axiolid-kernel}
packages/ifc/{ifc-schema,ifc-step,ifc-model,ifc-geometry,
              ifc-properties,ifc-cost,ifc-schedule}
packages/openbim/{ids,bcf,clash,diff}
bindings/{python,wasm}
apps/ifc-cli
```

Dependency direction is one-way and enforced by review:
`geometry → ifc → openbim → {bindings, apps}`. Nothing in `packages/ifc/` may
depend on `packages/openbim/`.

**Backends: features of `axiolid-kernel`, not separate crates.**

`axiolid-kernel` now holds both the traits and `backend::{scalar,simd,gpu}`, gated
by features `scalar` (default), `simd` (default), `gpu` (off).

**The swap boundary is re-expressed as a feature constraint.** Previously "no
`ifc/` crate names a backend crate". Now: *any `packages/ifc/` crate depending on
`axiolid-kernel` must set `default-features = false`* — the contract with no
implementation compiled in. Applications (`apps/ifc-cli`) opt back in explicitly.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep one crate per backend | Four manifests for four files; no isolation gained, since they share traits and data types either way. |
| Flat top-level `axiolid/`, `ifc/`, `bcf/`, `ids/`, … | Implies `bcf` is a peer of `axiolid`. It is a consumer; the layout should say so. |
| Backends as `#[cfg]` inside one module | Loses the ability to compile several backends at once, which kills differential testing (0002's core argument). |
| Keep `axiolid-dispatch` as its own crate | Its only job is selecting among backends it must all depend on. That is the definition of belonging with them. |

## Consequences

**Positive**

- One geometry-kernel manifest and one version to move.
- A consumer takes the traits with `default-features = false` and compiles
  literally zero backend code — verified: `cargo build -p axiolid-kernel
  --no-default-features` succeeds with no backend modules.
- Feature flags are visible in `cargo tree`/`cargo metadata`, so the boundary is
  machine-checkable rather than a naming convention.
- Room for `openbim/`, bindings, and apps without disturbing the core.

**Negative / costs**

- `axiolid-kernel` is now a larger crate holding both contract and implementations.
  Mitigated by the feature split: nothing is forced on a consumer.
- Feature unification is a real hazard. If any crate in a build enables
  `axiolid-kernel/simd`, every crate in *that build* gets it. This does not weaken
  the swap boundary (a foreign kernel is a different trait impl, unaffected) but
  it does mean "compiles no backend code" holds for a dependency graph, not for
  one crate inside a larger build that enables it elsewhere.

**Follow-ups / risks to watch**

- 🚨 **`default-features = false` on a workspace dependency is silently ignored
  unless the workspace's own `[workspace.dependencies]` entry also sets it.**
  Hit during this change: three manifests said `default-features = false` and
  Cargo warned it was being dropped, which would have made the entire boundary
  cosmetic. Fixed by setting it on the workspace entry and opting *in* at
  applications. The architecture test now covers this case.
- The `clash` crate is the load-bearing proof of the boundary outside `ifc/` —
  it is the heaviest geometry consumer and also takes contract-only.

## Relation to existing code

- `crates/axiolid-kernel/src/backend/{mod,scalar,simd,gpu}.rs`
- `packages/ifc/ifc-geometry/tests/no_backend_dependency.rs` — enforces the
  feature constraint; mutation-verified against two distinct violations.
- Root `Cargo.toml` — `axiolid-kernel` workspace entry carries
  `default-features = false`; `apps/ifc-cli` opts into `["scalar", "simd"]`.
