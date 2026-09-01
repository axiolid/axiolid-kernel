# 0021 — Capability seams live in `axiolid-kernel`; retire `axiolid-sweep`

- **Status:** Superseded by [0023](./0023-solid-generation-is-an-l2-crate.md)
- **Date:** 2026-08-30
- **Deciders:** Friedrich Schrödter
- **Supersedes:** —

## Supersession note

ADR 0021 was correct to retire the empty `axiolid-sweep` trait crate. Its
claim that sweep construction belonged in `axiolid-compile` was not measured
and was wrong: the construction modules form a model-free, dispatch-free L2
cluster. [ADR 0023](./0023-solid-generation-is-an-l2-crate.md) supersedes this
decision and extracts that cluster into `axiolid-construct`.

## Context

`axiolid-sweep` is a whole crate containing 30 lines: a module declaration and
one trait.

```rust
pub trait Sweeper: core::fmt::Debug + Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    fn sweep(
        &self,
        graph: &GeometryGraph,
        operation: &SolidOperation,
        tolerance: Tolerance,
    ) -> Result<TriMesh, Self::Error>;
}
```

Measured facts about it:

- **Zero implementors.** Nothing in the workspace implements `Sweeper`.
- **Zero external references.** `Sweeper` does not appear in any `.rs` file
  outside `crates/facade/axiolid-sweep/` itself.
- **Zero tests.** No `tests/` directory and no inline `#[cfg(test)]`.
- **The actual sweep implementation lives elsewhere**, in
  `crates/execution/compile/src/sweep.rs` (340 lines) and
  `crates/execution/compile/src/loft.rs` (200 lines), reached through the
  compiler rather than through this trait.

So the crate is a seam that nothing enters. Worse, it is *misleading*: the
facade exposes `axiolid::sweep` behind a `sweeps` feature, which reads as a
sweep capability but delivers only an unimplemented trait.

Compare the boolean seam, which is the pattern this project already uses
successfully:

| | `MeshBoolean` | `Sweeper` |
| --- | --- | --- |
| Contract lives in | `axiolid-kernel/src/boolean.rs` | its own crate |
| Implementors | `axiolid-mesh-boolean-boolmesh`, `axiolid-reference` | none |
| Conformance suite | `axiolid-kernel/src/conformance.rs` | none |
| Operand admissibility | `axiolid-kernel/src/solid.rs` | none |
| Referenced by | compiler, facade, providers | nothing |

`axiolid-kernel` is the established home for capability contracts: it holds the
backend trait, the boolean contract, solid admissibility, capability reporting,
cancellation, execution options, and the conformance suite. A capability seam
placed anywhere else is the outlier, and `Sweeper` is currently the only one.

There is also a layering reason. Sweeps need profiles and curves; a contract
crate that sits beside the kernel invites those dependencies into a layer that
should stay contract-only, whereas `axiolid-kernel` already gates such
dependencies behind features (`mesh-boolean`, `model`).

## Decision

We will **retire `axiolid-sweep`** and relocate the sweep capability seam into
`axiolid-kernel`, alongside `MeshBoolean`, when a sweep provider contract is
actually specified.

Concretely:

- Capability seams belong in `axiolid-kernel`. A new seam does not get its own
  crate unless it carries an implementation, and a crate that carries only a
  trait is a smell to be fixed rather than a pattern to copy.
- `axiolid-sweep` is removed rather than left as an empty promise. Removing an
  unused, unimplemented, untested crate is not a capability regression: nothing
  can depend on behaviour that never existed.
- The facade's `sweeps` feature and `axiolid::sweep` module are removed with
  it, because they advertise a capability the workspace does not provide.
- The sweep implementation stays in `axiolid-compile` for now. That is where
  the working code lives, and it is reachable through the compiler.
- When sweeps need to become provider-swappable, the trait is reintroduced in
  `axiolid-kernel` **together with** at least one implementor, a conformance
  suite, and operand admissibility rules — the same bar `MeshBoolean` meets.

This ADR does not change any sweep behaviour. `SolidOperation` and the
compiler's sweep families are untouched.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Leave the crate as an aspirational seam | It has been unreferenced since it landed, and the facade advertises it as a capability. An empty seam is worse than no seam: it implies swappability that does not exist. |
| Move the compiler's sweep code into `axiolid-sweep` | Plausible on the surface, but the sweep code depends on the compiler's evaluation context, profile resolution, and tolerance plumbing. Extracting it now would either drag those in or invert the dependency. Worth revisiting once ADR 0020's exact model settles what a sweep produces. |
| Implement `Sweeper` in `axiolid-compile` to justify the crate | Adds an indirection with exactly one implementor and no second candidate. The contract would be shaped by its only implementation, which is the failure mode ADR 0017 warns about for booleans. |
| Keep the crate, add tests to it | There is nothing to test. Tests over a trait with no implementors assert nothing. |

## Consequences

**Positive**

- One less crate, one less feature flag, and one fewer misleading capability in
  the facade's public surface.
- Capability seams become uniform: `axiolid-kernel` is the single place to look
  for what is swappable.
- The eventual sweep contract gets designed against a real second implementor
  and a conformance suite, rather than being frozen early around one.

**Negative / costs**

- Removing a public module and feature from the facade is a breaking change for
  any external caller that named them. At `0.1.0`, with no implementors behind
  the seam, the practical blast radius is nil, but it must appear in the
  changelog.
- Sweeps are temporarily not provider-swappable. They were never actually
  swappable, so this documents reality rather than changing it.

**Follow-ups / risks to watch**

- When the exact B-rep model from ADR 0020 lands, revisit what a sweep should
  return. A sweep of an exact profile along an exact directrix has an exact
  answer, and the reinstated contract should not be shaped around `TriMesh`.
- Risk: the same mistake recurs as new capabilities are sketched. Mitigation is
  the rule stated above — a seam ships with an implementor, or it does not ship.

## Relation to existing code

- `crates/facade/axiolid-sweep/` — removed by this decision.
- `crates/facade/axiolid/Cargo.toml`, `crates/facade/axiolid/src/lib.rs` — the `sweeps`
  feature and `pub mod sweep` re-export are removed.
- `Cargo.toml` — the workspace dependency entry is removed.
- `crates/contracts/common/src/boolean.rs` — the pattern a future sweep contract
  must follow.
- `crates/execution/compile/src/sweep.rs`, `crates/execution/compile/src/loft.rs` —
  where sweep construction actually lives; unchanged by this decision.
