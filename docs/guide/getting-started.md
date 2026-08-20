# Getting started

## Choose the narrowest dependency

Axiolid is a workspace, not a mandatory all-in-one dependency. Prefer a leaf crate when its public contract is sufficient; use the `axiolid` facade when the feature-gated composition is more convenient.

```toml
[dependencies]
# Core scalar values, transforms, bounds, and tolerance policy.
axiolid-core = { git = "https://github.com/axiolid/axiolid-kernel.git" }

# Or: a small facade with core values, meshes, and the portable CPU shell.
axiolid = { git = "https://github.com/axiolid/axiolid-kernel.git" }
```

The repository is currently consumed directly from Git while crates.io publication is not yet established. Pin a `rev` in reproducible applications.

## Start with core values

```rust
use axiolid::{Point3, Transform3};

let source = Point3::new(1.0, 2.0, 3.0);
let world = Transform3::IDENTITY.transform_point3(source);
assert_eq!(source, world);
```

`Point3`, `Vec3`, and transforms are double-precision `glam` aliases. Tolerance is explicit operation input; the kernel does not hide a global epsilon in geometry values.

## Opt into capabilities deliberately

```toml
# Mesh-oriented construction, triangulation, spatial operations, and the
# optional mesh-Boolean provider.
axiolid = { git = "https://github.com/axiolid/axiolid-kernel.git", features = ["discrete"] }

# Representation vocabulary for curves, surfaces, topology, and a neutral DAG.
axiolid = { git = "https://github.com/axiolid/axiolid-kernel.git", features = ["parametric"] }
```

| Bundle | Includes | Does not imply |
| --- | --- | --- |
| default | core values, mesh facade, portable CPU shell | every mesh algorithm |
| `discrete` | mesh-centric representations and declared algorithms/providers | exact B-rep evaluation |
| `parametric` | curve, surface, topology, primitive, and graph vocabulary | a complete CAD evaluator |
| `advanced` | `discrete` + `parametric` + healing vocabulary | production GPU computation |
| `full` | advanced facade plus optional parallel/SIMD/GPU seams | that each acceleration path is implemented or faster |

Read [Capabilities](/capabilities) before basing product behavior on a feature.

## Build and test locally

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

For the architecture-specific feature matrix and mutation probes, see [Contributing](/guide/contributing).
