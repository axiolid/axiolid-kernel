# Architecture decisions

These records capture durable choices that preserve Axiolid’s format-neutral, pure-Rust, feature-tiered design. A decision record explains its context, boundary, alternatives, consequences, and follow-up risks; it is not a changelog.

| ADR | Decision |
| --- | --- |
| [0001](./0001-axiolid-ifc-split-and-kernel-contract.md) | IFC split and kernel contract |
| [0002](./0002-hardware-abstraction-and-backend-selection.md) | Hardware abstraction and backend selection |
| [0003](./0003-pure-rust-mesh-boolean.md) | Pure-Rust mesh Boolean direction |
| [0004](./0004-package-layout-and-backend-features.md) | Package layout and backend features |
| [0009](./0009-layered-geometry-dag.md) | Layered geometry DAG and operation providers |
| [0011](./0011-native-accelerator-backends-out-of-tree.md) | Native accelerator backends out of tree |
| [0012](./0012-scalar-reference-ownership.md) | Scalar reference ownership |
| [0013](./0013-deferred-performance-techniques.md) | Deferred performance techniques |
| [0014](./0014-adopt-boolmesh-mesh-boolean.md) | Adopt Boolmesh as mesh Boolean provider |
| [0015](./0015-adopt-earcut-polygon-triangulation.md) | Adopt Earcut for polygon triangulation |
| [0016](./0016-predicate-ownership-and-adopted-implementations.md) | Predicate ownership and adopted implementations |
| [0017](./0017-solid-boolean-contract-before-implementation.md) | Solid boolean semantics defined before implementation |
| [0018](./0018-curve-evaluation-in-the-scalar-reference.md) | Curve evaluation belongs to the scalar reference |
| [0019](./0019-validate-and-refine-nurbs-on-the-scalar-read-path.md) | Validate and refine NURBS on the scalar read path |
| [0020](./0020-exact-brep-kernel-model.md) | Exact B-rep is the kernel model; tessellation is an output |
| [0021](./0021-capability-seams-live-in-the-kernel.md) | ~~Capability seams live in `axiolid-kernel`; retire `axiolid-sweep`~~ — superseded by 0023 |
| [0022](./0022-general-nurbs-kernel-capability.md) | General NURBS algorithms are a kernel capability |
| [0023](./0023-solid-generation-is-an-l2-crate.md) | Solid generation is an L2 crate, not part of the compiler |
| [0024](./0024-exact-brep-result-contracts.md) | Exact B-rep results own analytic supports and trims |
| [0025](./0025-certified-nurbs-subdivision-oracle.md) | Certified NURBS queries use outward-rounded subdivision |
| [0026](./0026-certified-planar-nurbs-root-isolation.md) | Planar NURBS roots require interval existence proofs |
| [0027](./0027-certified-nurbs-curve-surface-root-isolation.md) | Curve/surface roots require bounded transverse existence proofs |
| [0028](./0028-certified-affine-surface-surface-tracing.md) | Affine surface/surface traces compose exact identities and certified boundary roots |
| [0029](./0029-certified-trace-topology-integration.md) | Certified finite traces become split faces plus embedded curves |
| [0030](./0030-globally-certified-surface-projection.md) | Surface closest-point certificates require bounded global branch-and-bound |
| [0031](./0031-verified-periodic-curve-views.md) | Periodic curve behavior is an opt-in verified view |
| [0032](./0032-explicit-periodic-bspline-surfaces.md) | Periodic B-spline surfaces use an explicit cyclic schema |
| [0033](./0033-mesh-plane-section-contract.md) | Mesh plane sections are an explicit approximation tier |
| [0034](./0034-authored-open-profile-contract.md) | Authored open profiles are graph declarations, not areas |
| [0035](./0035-nested-ownership-and-capability-contracts.md) | Physical layout follows ownership; contracts remain provider-neutral |
