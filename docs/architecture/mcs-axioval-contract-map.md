# MCS / Axioval integration contract

Status: authoritative Axiolid-side boundary. The MCS/Pkl and Protobuf schemas are external adapters and are not vendored into geometry packages.

## Capability map

| Stable ID | Rust contract | Requirements | Guarantees / evidence | Provider / execution |
| --- | --- | --- | --- | --- |
| `org.axiolid.geometry.tessellate.v1` | `axiolid_tessellation_contract::Tessellator` | typed geometry graph/root and validated `TessellationOptions` | triangle mesh under explicit request parameters; no exact-B-rep claim | provider chosen outside the contract |
| `org.axiolid.geometry.mesh-boolean.v1` | `axiolid_mesh_boolean_contract::MeshBoolean` | oriented closed mesh operands, operator, execution options | `BooleanOutcome` + `BooleanEvidence`; conformance suite supplied by the contract | providers such as `axiolid-mesh-boolean-boolmesh`; optional `axiolid-dispatch` registry |
| `org.axiolid.geometry.mesh-section.v1` | `axiolid_mesh_section_contract::MeshPlaneSection` | oriented closed mesh, plane frame, typed section limits/options | closed plane-local contours + `SectionEvidence`; unsupported topology fails closed | reference provider in `axiolid-reference`; optional dispatch registry |
| `org.axiolid.geometry.graph-to-mesh.v1` | `axiolid_mesh_compile_contract::MeshCompiler` | geometry graph/root and execution options | explicitly mesh-valued output; never an implicit exact-B-rep result | `axiolid-mesh-compile` reference execution pipeline or GPU adapter |

The stable constants are typed `axiolid_contracts::CapabilityId` values. IDs version semantic contracts, not implementations, runtime plans, Pkl records, or Protobuf messages.

## Concept mapping

| MCS / Pkl concept | Axiolid owner |
| --- | --- |
| `Capability` | stable `CapabilityId` plus an operation trait |
| `Requirements` | typed request arguments, limits, admissibility and execution options |
| `Guarantees` | `axiolid-guarantees`, operation result types, and documented invariants |
| `Evidence` | operation-specific typed evidence; missing evidence is not success |
| `Diagnostics` | `GeomError`, `Operation`, backend identity, and bounded output diagnostics |
| `Provider` | a concrete package implementing exactly one or more operation traits |
| `ExecutionPlan` | internal to dispatch/execution; never serialized as a portable capability schema |
| `Conformance` | operation-owned suites and mutation-verified repository gates |

Pkl/Protobuf adapters must lower source records into these typed Rust contracts. They must not move Pkl, Protobuf, IFC, CSET, or `Any` payloads into representation or contract packages. Recursive source ASTs must be bounded and normalized before reaching Axiolid.

## Provider obligations

A provider declaration is valid only when all are true:

1. the stable capability ID and version match the implemented Rust trait;
2. requirements are admitted before execution;
3. unsupported inputs or missing evidence produce explicit refusal/indeterminate diagnostics;
4. conformance evidence identifies the suite/vector version and provider build;
5. device, memory, scheduling, retry, fallback, and provider ordering remain execution metadata—not capability semantics.

A provider name, registry entry, or protobuf message alone is never capability evidence.

## Semantic package identity

Package identity is computed from the canonical semantic model, never from encoded Protobuf bytes.

The canonical digest input must contain, in deterministic field order:

- semantic schema/version identifier;
- stable capability IDs and contract versions;
- normalized requirements and guarantee declarations;
- normalized evidence references and conformance-vector identities;
- explicitly versioned unknown/extension fields when the semantic schema preserves them.

Map/set ordering, float representation, units, omitted/default distinctions, Unicode normalization, and unknown-field policy must be specified by the external adapter before hashing. Equivalent protobuf encodings, field orderings, unknown transport fields, or compression choices must not change semantic identity. Conversely, a semantic requirement/guarantee change must change identity even when a transport encoder happens to emit similar bytes.

If canonicalization cannot represent a value or preserved extension unambiguously, identity generation fails closed. It must not silently hash raw transport bytes as a fallback.

## Conformance vectors

Each vector binds:

- capability ID/version;
- typed input and expected result/refusal class;
- tolerance/limits and required evidence predicates;
- deterministic vector ID and semantic digest;
- provider-independent diagnostics expected on unsupported cases.

Provider-specific acceleration metadata may accompany a run report, but it is not part of the portable vector semantics.
