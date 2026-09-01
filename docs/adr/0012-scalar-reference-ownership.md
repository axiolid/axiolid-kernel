# 0012 — The scalar reference is a real crate, not a doctrine

- **Status:** Accepted
- **Date:** 2026-08-19
- **Deciders:** Friedrich, axiolid
- **Supersedes:** the backend topology and oracle-ownership parts of 0002

## Context

R10 says the scalar backend is the reference implementation, replay mechanism,
debugging path, and portability baseline -- not disposable scaffolding.

ADR 0002 already asserts that doctrine: *"`axiolid-cpu` is the correctness oracle:
portable, no intrinsics, always available."* The doctrine is right. The topology
it describes is stale and the ownership is unassigned.

Measured against the tree at the time of writing:

- `axiolid-cpu`, `axiolid-simd`, `axiolid-gpu`, `axiolid-dispatch` do not exist. The crates
  are `axiolid-backend-cpu` and `axiolid-backend-gpu`.
- Every path in ADR 0002's "Relation to existing code" is dead.
- `axiolid-backend-cpu` is an execution *context*: ISA detection, a Rayon pool, a
  builder. Its own docs say it "bundles no SIMD algorithm" -- and it bundles no
  scalar algorithm either.

The named oracle therefore owns no algorithm, and nothing is validated against
anything. If an optimized path lands before a scalar reference exists, both R3
(differential testability) and R10 become unenforceable, quietly.

## Decision

The scalar reference is a **named crate that owns algorithms**, distinct from
the execution context that schedules them.

- `axiolid-backend-cpu` stays what it is: an execution *context* (ISA detection,
  worker pool, policy). It is not the oracle and must not be called one.
- The scalar reference implementation gets its own crate, `axiolid-reference`, which
  owns portable algorithms with no intrinsics, no `unsafe`, and no threading.
  It is the oracle, the replay path, and the portability baseline.
- Ordering is inverted from ADR 0002's implied schedule: **the scalar
  implementation of an operation lands before any optimized implementation of
  that operation.** An optimized path without a scalar counterpart cannot be
  differentially tested, so it does not ship.
- The oracle is never feature-gated off. Any build that can run an operation can
  also run its reference.

ADR 0002's *reasoning* (runtime selection over compile-time `#[cfg]`, single
portable binary, differential validation) stands unchanged. Only its crate
topology and its oracle assignment are superseded.

## Amendment 2026-08-26 — the ordering rule was violated, and repaired

The rule above ("the scalar implementation of an operation lands before any
optimized implementation of that operation") was **not** honoured for mesh
booleans. `axiolid-mesh-boolean-boolmesh` was adopted under ADR 0014 while `axiolid-reference`
implemented no `MeshBoolean` at all. For a period, the single most consequential
operation in the kernel had no oracle, and its only test suite bound the
adopted provider's concrete type — so it could confirm that the provider agreed
with itself, and nothing more.

Two repairs were considered:

| Option | Why not |
| --- | --- |
| Record a scoped exception naming `boolmesh` as a pre-oracle adoption | Cheap, and permanently exempts the operation that most needs the rule. An exception written once is a precedent: the next adoption cites it, and the rule decays into a preference. It also leaves the actual problem — no independent check on boolean geometry — unsolved. |
| **Write the oracle (chosen)** | Restores the invariant instead of documenting its absence. Costs one `O(n·m)` reference implementation, which is exactly what the rule always asked for. |

The exception was rejected because the rule's value is that it has no
exceptions. A conformance suite validated against a single implementation
cannot distinguish *correct* from *self-consistent*, and that distinction is
the whole reason ADR 0012 exists.

`axiolid_reference::ScalarBoolean` now implements `MeshBoolean` (ADR 0017 §5). It
is exact where it answers and refuses where it cannot, which preserves the
oracle's defining property: it is never *approximately* right.

### The rule, restated operationally

An ordering rule that lives only in prose gets violated silently — as this one
was. It is now enforced by construction:

- `axiolid_kernel::conformance` is generic over `impl MeshBoolean`, so
  obligations attach to the contract rather than to a provider.
- `MeshBooleanRegistry::register_conformant` refuses a non-conformant provider
  at registration, returning the failing report.
- `axiolid-mesh-boolean-boolmesh/tests/conformance.rs` runs the oracle and the production
  provider through the identical suite and compares their geometry directly.

An operation that acquires an optimized provider without a scalar counterpart
now has no way to demonstrate conformance, because there is nothing to be
differentially tested against. The rule enforces itself.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep the oracle inside `axiolid-backend-cpu` | Conflates scheduling policy with algorithm ownership. The context crate would then need `parallel`/`simd` features on the very code that must stay portable and single-threaded to be a trustworthy reference. |
| Treat the scalar path as a fallback that may be dropped later | Exactly the "disposable scaffolding" R10 forbids. Once dropped, no optimized path can be validated and no bug can be replayed portably. |
| Rewrite ADR 0002 in place | ADRs are a decision log. Editing history hides that the topology changed and why. |
| Leave it as doctrine until algorithms exist | The gap is invisible precisely while there are no algorithms, which is when the ordering rule needs to be established. |

## Consequences

**Positive**

- Every optimized path has a named, always-present counterpart to be tested
  against, so R3 and R10 are enforceable rather than aspirational.
- A portable replay path exists for debugging a SIMD or GPU discrepancy.
- The scalar crate has no features, no threads, and no intrinsics, so it is
  trivially portable to AArch64, WASM, and any future target.

**Negative / costs**

- Every operation is written at least twice. This cost was already accepted in
  ADR 0002 and is restated here, not introduced.
- One more crate in the workspace.

**Follow-ups / risks to watch**

- `axiolid-reference` does not exist yet. This ADR sets the rule that governs its
  creation; it must be created with the first geometry algorithm, not after.
- The risk this ADR exists to prevent: an AVX-512 or GPU path landing first
  because it is more interesting to write. The ordering rule above is the guard.

## Relation to existing code

- `crates/execution/cpu/` -- execution context; explicitly not the
  oracle.
- `crates/contracts/common/src/capability.rs` -- `ExecutionTarget::PortableCpu`
  is the target a reference implementation reports.
- `docs/adr/0002-hardware-abstraction-and-backend-selection.md` -- reasoning
  retained, topology and oracle ownership superseded here.
