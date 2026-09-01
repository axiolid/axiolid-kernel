//! Executable gate: a third-party accelerator backend can be built entirely
//! out of tree.
//!
//! The roadmap defers a native CUDA/HIP executor, and ADR 0009 states such a
//! backend arrives as an out-of-tree crate implementing this seam. That promise
//! is only real if the seam can actually be satisfied by a crate with **no
//! privileged access**: no `pub(crate)` helpers, no internal modules, no
//! `#[doc(hidden)]` constructors — only the published API surface.
//!
//! This integration test lives outside `src/`, so it links `axiolid_backend_gpu`
//! exactly as an external crate does. Anything it needs and cannot reach would
//! also be unreachable for a real bridge crate. A `SimulatedNativeExecutor`
//! stands in for a FFI-backed executor: it models device identity, driver
//! ordinals, unwind isolation, and failure attribution without requiring a GPU.

use std::panic::{catch_unwind, AssertUnwindSafe};

use axiolid_backend_gpu::{GpuCompiler, GpuDeviceDescriptor, GpuFeatures, GpuGraphExecutor};
use axiolid_contracts::{
    Backend, BackendId, DevicePreference, ExecutionOptions, ExecutionTarget, GeomError, GeomResult,
    Operation, Precision,
};
use axiolid_core::{Point3, Tolerance};
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryGraph, GeometryGraphBuilder, GeometryNode, NodeId};

/// Stand-in for a driver-backed executor (CUDA, HIP, Level Zero).
///
/// It deliberately uses only the public API, and constructs its identity from a
/// *runtime* ordinal the way a real driver enumeration would.
#[derive(Debug)]
struct SimulatedNativeExecutor {
    device: GpuDeviceDescriptor,
    /// When set, `compile_mesh_batch` panics to model a native callback that
    /// unwinds. A real bridge must never let this cross an FFI frame.
    panic_on_compile: bool,
}

impl SimulatedNativeExecutor {
    fn new(api: &str, ordinal: u32, float64: bool) -> Self {
        // Exactly the pattern a driver bridge uses: identity is only known once
        // the device has been enumerated at runtime.
        let id = BackendId::try_new(&format!("{api}:{ordinal}"))
            .expect("driver-enumerated identity must fit BackendId::CAPACITY");
        Self {
            device: GpuDeviceDescriptor {
                id,
                name: format!("Simulated {api} device {ordinal}"),
                api: api.to_owned(),
                features: GpuFeatures {
                    float64,
                    subgroups: true,
                    unified_memory: false,
                    max_workgroup_size: 1024,
                },
            },
            panic_on_compile: false,
        }
    }
}

impl GpuGraphExecutor for SimulatedNativeExecutor {
    fn device(&self) -> &GpuDeviceDescriptor {
        &self.device
    }

    fn validate_options(&self, options: &ExecutionOptions) -> GeomResult<()> {
        // A native backend rejects policies its driver cannot honour rather
        // than silently degrading them.
        if options.precision() == Precision::F64 && !self.device.features.float64 {
            return Err(GeomError::Unsupported {
                backend: self.device.id,
                operation: Operation::GraphCompilation,
            });
        }
        Ok(())
    }

    fn compile_mesh_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        _options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        // Unwind isolation: a real bridge wraps the foreign call so a panic
        // never crosses an FFI frame (that is undefined behaviour). The
        // simulation performs the same containment.
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            assert!(!self.panic_on_compile, "simulated driver fault");
            roots
                .iter()
                .map(|root| {
                    // Prove the graph is walkable through the public API alone.
                    let _ = graph.get(*root);
                    TriMesh::new(Vec::<Point3>::new(), Vec::new())
                })
                .collect::<Vec<_>>()
        }));

        outcome.map_err(|_| GeomError::BackendContractViolation {
            backend: self.device.id,
            detail: "native call unwound; contained at the bridge boundary".to_owned(),
        })
    }
}

fn single_point_graph() -> (GeometryGraph, NodeId) {
    let mut builder = GeometryGraphBuilder::new();
    let node = builder
        .push(GeometryNode::Point3(Point3::new(0.0, 0.0, 0.0)))
        .expect("point node");
    let graph = builder.finish(vec![node]).expect("graph");
    (graph, node)
}

#[test]
fn an_out_of_tree_executor_satisfies_the_seam_with_public_api_only() {
    let executor = SimulatedNativeExecutor::new("cuda", 0, true);
    let compiler = GpuCompiler::new(executor);
    let (graph, root) = single_point_graph();

    let descriptor = compiler.descriptor();
    assert_eq!(descriptor.target, ExecutionTarget::Gpu);
    assert_eq!(descriptor.id.as_str(), "cuda:0");

    let meshes = compiler
        .compile_mesh_batch(&graph, &[root], &ExecutionOptions::new(Tolerance::METRE))
        .expect("simulated native compile");
    assert_eq!(meshes.len(), 1);
}

/// Driver-enumerated identities are runtime values. If `BackendId` regressed to
/// requiring `&'static str`, a real bridge could only comply by leaking memory.
#[test]
fn driver_enumerated_devices_get_distinct_runtime_identities() {
    let first = SimulatedNativeExecutor::new("hip", 0, true);
    let second = SimulatedNativeExecutor::new("hip", 1, true);
    assert_ne!(first.device().id, second.device().id);
    assert_eq!(first.device().id.as_str(), "hip:0");
    assert_eq!(second.device().id.as_str(), "hip:1");

    // Explicit selection by runtime identity must round-trip through the
    // public execution policy.
    let options = ExecutionOptions::new(Tolerance::METRE).with_device(DevicePreference::Backend(
        BackendId::try_new("hip:1").expect("identity"),
    ));
    let compiler = GpuCompiler::new(SimulatedNativeExecutor::new("hip", 1, true));
    let (graph, root) = single_point_graph();
    assert!(compiler.compile_mesh(&graph, root, &options).is_ok());

    let mismatched = GpuCompiler::new(SimulatedNativeExecutor::new("hip", 0, true));
    assert!(matches!(
        mismatched.compile_mesh(&graph, root, &options),
        Err(GeomError::Unsupported { backend, .. }) if backend.as_str() == "hip:0"
    ));
}

/// A panic inside the simulated native call must surface as a typed backend
/// error, never as an unwind escaping the boundary.
#[test]
fn a_faulting_native_call_is_contained_and_attributed() {
    let mut executor = SimulatedNativeExecutor::new("cuda", 0, true);
    executor.panic_on_compile = true;
    let compiler = GpuCompiler::new(executor);
    let (graph, root) = single_point_graph();

    let error = compiler
        .compile_mesh(&graph, root, &ExecutionOptions::new(Tolerance::METRE))
        .expect_err("a faulting driver call must produce an error, not unwind");
    assert!(
        matches!(
            error,
            GeomError::BackendContractViolation { backend, .. } if backend.as_str() == "cuda:0"
        ),
        "expected contained backend fault, got {error:?}"
    );
}

/// Unsupported precision must be refused by the executor's own policy hook
/// before any work is submitted, and blamed on the backend that refused it.
#[test]
fn an_f32_only_device_refuses_f64_before_submission() {
    let compiler = GpuCompiler::new(SimulatedNativeExecutor::new("hip", 0, false));
    let (graph, root) = single_point_graph();
    let options = ExecutionOptions::new(Tolerance::METRE).with_precision(Precision::F64);

    assert!(matches!(
        compiler.compile_mesh(&graph, root, &options),
        Err(GeomError::Unsupported {
            backend,
            operation: Operation::GraphCompilation,
        }) if backend.as_str() == "hip:0"
    ));
}

/// The seam must not require a GPU-specific crate to be a workspace member. If
/// this test compiles and links using only published items, an out-of-tree
/// crate can do the same.
#[test]
fn the_seam_requires_no_privileged_access() {
    fn accepts_any_executor<E: GpuGraphExecutor>(executor: E) -> BackendId {
        GpuCompiler::new(executor).descriptor().id
    }
    let id = accepts_any_executor(SimulatedNativeExecutor::new("level-zero", 3, true));
    assert_eq!(id.as_str(), "level-zero:3");
}
