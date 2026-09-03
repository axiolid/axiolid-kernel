# 0039 — Versioned downstream integration contracts

- **Status:** Accepted
- **Date:** 2026-09-03
- **Deciders:** Friedrich Schrödter
- **Supersedes:** —

## Context

Axiolid's operation traits, provider descriptors, feature-isolated facade, and closure gates are strong internal seams. They do not by themselves tell an application which consumption route is supported, how to discover the capabilities compiled into one build, or which unit, tolerance, fidelity, ownership, and concurrency rules cross that boundary.

This ambiguity is especially dangerous for native consumers: a caller could mistake package version for ABI compatibility, free memory with the wrong allocator, race a mutable context, or silently accept a mesh where an exact B-rep was requested. The integration contract must remain format-neutral and must not make declarations more authoritative than concrete operation traits and conformance evidence.

## Decision

Axiolid v0.4 defines a versioned integration protocol and three named profiles: `rust-leaf`, `rust-facade`, and `native-c`.

- `rust-leaf` selects publishable leaf packages directly. Cargo features and implemented operation traits are compile-time discovery; `Backend::descriptor` identifies concrete runtime providers.
- `rust-facade` selects additive capabilities on the `axiolid` package. The built facade exposes an `IntegrationDescriptor` derived from enabled features and actually registered providers.
- `native-c` crosses a dedicated C ABI using opaque handles and matching Axiolid destructors. It exposes an ABI-version query and the same capability descriptor without leaking Rust layouts.
- The integration protocol starts at `0.4.0`. Compatibility requires the same protocol major and an offered minor/patch no older than requested. Package SemVer and ABI compatibility remain separate promises.
- Capability identifiers are stable versioned vocabulary. Their presence in the vocabulary is not an availability claim. A build advertises a `CapabilityDescriptor` only when a concrete implementation of the corresponding operation trait is present and executable evidence covers it.
- Every descriptor names its input representations, output representation, provider, admitting feature, fidelity, and determinism. Exact and tolerance-bounded outputs are distinct.
- `IntegrationDescriptor::require` returns a typed refusal when API version, capability, representation, or fidelity is unavailable. It never selects another provider and never degrades exact work to a mesh.
- All coordinates are right-handed Cartesian `f64`. Geometry is unitless; coordinates and tolerances use one caller-selected unit. Approximate/discretizing operations require explicit tolerance.
- Rust values own their allocations and are `Send + Sync` where their public types guarantee it. Native outputs use opaque owned handles; contexts may move between threads but are not concurrently usable unless a future descriptor explicitly strengthens that contract.
- Identical inputs, options, provider/version, and execution target must be deterministic before the descriptor advertises determinism. Parallel/GPU providers may honestly report weaker guarantees.

The generated [downstream integration profile table](../architecture/downstream-integration.md) is the public summary. A freshness gate compares it byte-for-byte with executable Rust definitions, and a mutation probe removes a promised capability identifier to prove that drift is rejected.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Treat facade Cargo features as the contract | Native consumers cannot inspect Cargo features, and a feature can expose a schema without an executable provider. |
| Make backend metadata the sole truth | `BackendDescriptor` identifies execution targets but does not encode boundary ownership, protocol compatibility, representation fidelity, or build features. |
| Return a generic "unsupported" string | Callers could not reliably distinguish version, capability, representation, and exactness failures. |
| Automatically fall back from exact to mesh | This changes geometry identity and editability and would make capability claims dishonest. |
| Freeze the eventual 1.0 API now | v0.4 needs a usable compatibility handshake without claiming the wider surface is stable. |

## Consequences

**Positive**

- Rust and native surfaces share one neutral compatibility vocabulary.
- Applications can refuse unsupported configurations before submitting geometry.
- Exactness and tolerance are visible product behavior rather than provider accidents.
- Documentation drift is executable and mutation-tested.

**Negative / costs**

- Every new operation exposed downstream needs descriptor evidence in addition to its operation trait and conformance suite.
- Pre-1.0 protocol evolution must preserve or explicitly version compatibility behavior.
- Native bindings require a dedicated unsafe boundary even though the rest of the workspace forbids unsafe code.

**Follow-ups / risks to watch**

- The facade and C ABI must populate descriptors from actually linked providers, not wish-list metadata.
- Cross-platform probes must verify allocator ownership and context concurrency behavior.
- Descriptor serialization, if added, must be explicitly versioned and must preserve unknown fields or fail closed.

## Relation to existing code

- `crates/contracts/common/base/src/integration.rs` owns the provider-neutral descriptor and typed handshake.
- `crates/contracts/common/base/src/profiles.rs` owns the three v0.4 profile promises.
- `crates/contracts/common/base/src/capability_id.rs` owns stable capability identifiers.
- `docs/architecture/downstream-integration.md` is generated from executable definitions.
- `scripts/check-integration-contract.sh` and `scripts/probe_integration_contract_gate.py` enforce freshness and mutation sensitivity.
