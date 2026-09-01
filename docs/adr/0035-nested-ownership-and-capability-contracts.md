# ADR 0035: Nested ownership layout and portable capability contracts

- Status: Accepted
- Date: 2026-09-01

## Context

Axiolid has sound lower-level decisions—format neutrality, exact-intent preservation, independently consumable representations, portable reference oracles, swappable providers, and fail-closed contracts—but its flat `crates/facade/axiolid-*` filesystem obscures those roles. The mixed `axiolid-kernel` package also places guarantee vocabulary, provider/execution policy, operation seams, registries, evidence, and graph-to-mesh compilation behind one dependency boundary.

ADR 0020 already decides that exact B-rep is the kernel model and tessellation is an explicit output. The former `GeometryCompiler -> TriMesh` contract contradicted that direction by making a discrete result the universal graph currency.

The broader capability ecosystem needs stable operation semantics which can be mapped to external capability claims without importing Pkl, Protobuf, IFC, vendor, transport, or source-format types into Axiolid.

## Decision

Axiolid adopts a physically nested Cargo workspace organized by ownership:

- `foundation` for root values;
- `representations` for independently useful data models;
- `contracts` for guarantee vocabulary, common provider/execution requests, and real operation seams;
- `algorithms` for portable implementation logic, including the reference oracle;
- `providers` for replaceable adopted/native implementations;
- `execution` for orchestration and hardware/runtime contexts;
- `facade` for additive convenience features.

Folders, packages, and modules do not mirror one another one-to-one. A package exists only for a real dependency, trust, provider, licensing, conformance, platform, or meaningful compilation boundary.

Every workspace package declares `[package.metadata.axiolid]` ownership metadata. An executable architecture checker validates package placement, metadata, format neutrality, and declared dependency edges. Generated documentation comes from Cargo metadata. The checker and its mutation probes replace the coarse hard-coded tier table once equivalent and stronger behavior is demonstrated.

`axiolid-kernel` is decomposed by ownership:

- representation-independent guarantees;
- common provider/execution contract infrastructure;
- operation-specific seams for existing real capabilities.

Operation contracts own stable capability identity/version, requirements, result/evidence/diagnostic semantics, and conformance interfaces. They never depend on concrete providers. Runtime provider selection cannot redefine operation meaning.

Graph traversal, caching, batching, and provider resolution remain orchestration. Graph-to-mesh is explicit tessellation; it is not universal geometry compilation. No universal result enum is introduced. Future exact operations receive contracts only with a real implementation or independent consumer.

The portable scalar implementation remains an algorithmic reference oracle and is renamed accordingly. Constructive modeling and adopted provider package names are made explicit. Sampled-field stored values are separated from sampled-field algorithms.

## Dependency policy

The checker enforces a role-based DAG over production/build edges and an exact declared allowlist over all internal edges, including dev integration dependencies. At minimum:

- foundation has no Axiolid sibling dependency;
- representations do not depend on contracts, algorithms, providers, execution, facade, or formats;
- contracts do not depend on algorithms, providers, execution, facade, or formats;
- algorithms do not depend on providers, execution, facade, or formats;
- providers do not depend on execution selection or facade;
- execution may orchestrate contracts/algorithms/providers but may not redefine semantics;
- facade may depend on any public Axiolid package;
- source-format and vendor types are forbidden from all core packages.

The architecture gate enforces the maintained forbidden-term catalog across
production `src/**/*.rs` files. Tests may name those formats only to verify
rejection and adapter boundaries; test vocabulary is not shipped library code.

Narrow representation composition and lower algorithm-substrate edges are explicitly declared rather than inferred from a numeric layer.

## Capability boundary

External MCS/Axioval or wire packages map normalized semantic requests into typed Axiolid operation requests. Axiolid contracts expose no Pkl, Protobuf, IFC, Revit, Solibri, CSET, or source-format types. Missing geometry, unsupported representations, unavailable evidence, and provider absence are typed non-success outcomes, never successful negatives.

## Consequences

### Positive

- Filesystem navigation communicates ownership.
- Small consumers retain leaf-crate dependency paths.
- Operation semantics become portable across providers and execution targets.
- Exact/discrete result domains are explicit.
- Architecture drift is machine-checked and documented from Cargo metadata.
- Provider, unsafe, native, and licensing boundaries are visible.

### Negative and migration costs

- Package paths and several package names change.
- Published crate renames require explicit downstream migration; crates.io has no true rename.
- Contract decomposition touches most algorithm/provider imports.
- Feature and conformance matrices grow.
- Physical moves create a large rename diff even when behavior is unchanged.

## Rejected alternatives

- **Keep the flat workspace:** preserves paths but leaves role ownership implicit.
- **One crate per taxonomy node:** creates shallow microcrates and release overhead without independent boundaries.
- **One physical mega-crate:** weakens Cargo-enforced dependency and provider boundaries.
- **Move the mixed kernel unchanged:** improves appearance while preserving the actual ownership defect.
- **Universal geometry-result enum:** hides operation semantics and exact/discrete refusal policy behind a broad sum type.

## Verification

The migration must preserve workspace, feature, conformance, mutation, documentation, and downstream IFC gates. Meaningful splits report dependency/build measurements; claims of improvement require evidence.
