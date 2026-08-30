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
| [0021](./0021-capability-seams-live-in-the-kernel.md) | Capability seams live in `axiolid-kernel`; retire `axiolid-sweep` |
