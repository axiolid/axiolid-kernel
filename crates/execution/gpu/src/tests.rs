use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axiolid_contracts::{
    Backend, BackendId, DataResidency, DevicePreference, ExecutionOptions, GeomError, GeomResult,
    Operation, Residency,
};
use axiolid_core::{Tolerance, Vec3};
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryGraph, GeometryGraphBuilder, GeometryNode, NodeId};

use super::*;

#[derive(Debug)]
struct FakeExecutor {
    device: GpuDeviceDescriptor,
}

#[derive(Debug)]
struct RejectingOptionsExecutor {
    inner: FakeExecutor,
    compile_calls: Arc<AtomicUsize>,
}

impl GpuGraphExecutor for FakeExecutor {
    fn device(&self) -> &GpuDeviceDescriptor {
        &self.device
    }

    fn validate_options(&self, _options: &ExecutionOptions) -> GeomResult<()> {
        Ok(())
    }

    fn compile_mesh_batch(
        &self,
        _graph: &GeometryGraph,
        roots: &[NodeId],
        _options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        Ok(roots.iter().map(|_| TriMesh::default()).collect())
    }
}

#[derive(Debug)]
struct WrongCardinalityExecutor(FakeExecutor);

impl GpuGraphExecutor for WrongCardinalityExecutor {
    fn device(&self) -> &GpuDeviceDescriptor {
        self.0.device()
    }

    fn validate_options(&self, options: &ExecutionOptions) -> GeomResult<()> {
        self.0.validate_options(options)
    }

    fn compile_mesh_batch(
        &self,
        _graph: &GeometryGraph,
        _roots: &[NodeId],
        _options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        Ok(vec![TriMesh::default(), TriMesh::default()])
    }
}

impl GpuGraphExecutor for RejectingOptionsExecutor {
    fn device(&self) -> &GpuDeviceDescriptor {
        self.inner.device()
    }

    fn validate_options(&self, _options: &ExecutionOptions) -> GeomResult<()> {
        Err(GeomError::Unsupported {
            backend: self.inner.device.id,
            operation: Operation::GraphCompilation,
        })
    }

    fn compile_mesh_batch(
        &self,
        _graph: &GeometryGraph,
        _roots: &[NodeId],
        _options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        self.compile_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Vec::new())
    }
}

#[test]
fn downstream_executor_proves_its_capability_by_implementing_the_trait() {
    let executor = FakeExecutor {
        device: GpuDeviceDescriptor {
            id: BackendId::new("test-gpu-compiler"),
            name: "test".to_owned(),
            api: "mock".to_owned(),
            features: GpuFeatures {
                float64: true,
                subgroups: false,
                unified_memory: true,
                max_workgroup_size: 64,
            },
        },
    };
    let compiler = GpuCompiler::new(executor);
    let mut builder = GeometryGraphBuilder::new();
    let root = builder
        .push(GeometryNode::Point3(Vec3::ZERO))
        .expect("root");
    let graph = builder.finish(vec![root]).expect("graph");
    let options = ExecutionOptions::new(Tolerance::METRE);
    assert_eq!(
        compiler
            .compile_mesh(&graph, root, &options)
            .expect("compile"),
        TriMesh::default()
    );
    assert_eq!(
        compiler.descriptor().id,
        BackendId::new("test-gpu-compiler")
    );

    let mut f32_device = compiler.device().clone();
    f32_device.id = BackendId::new("f32-only");
    f32_device.features.float64 = false;
    let f32_only = GpuCompiler::new(FakeExecutor { device: f32_device });
    assert!(matches!(
        f32_only.compile_mesh(&graph, root, &options),
        Err(GeomError::Unsupported {
            backend,
            operation: Operation::GraphCompilation,
        }) if backend == BackendId::new("f32-only")
    ));
}

#[test]
fn adapter_rejects_foreign_roots_before_dispatch() {
    let compiler = GpuCompiler::new(fake_executor("root-validation", true));
    let (graph, _) = point_graph(Vec3::ZERO);
    let (_, foreign_root) = point_graph(Vec3::ONE);

    assert!(matches!(
        compiler.compile_mesh(
            &graph,
            foreign_root,
            &ExecutionOptions::new(Tolerance::METRE),
        ),
        Err(GeomError::InvalidInput { .. })
    ));
}

#[test]
fn adapter_enforces_one_result_per_requested_root() {
    let compiler = GpuCompiler::new(WrongCardinalityExecutor(fake_executor(
        "wrong-cardinality",
        true,
    )));
    let (graph, root) = point_graph(Vec3::ZERO);

    assert!(matches!(
        compiler.compile_mesh_batch(&graph, &[root], &ExecutionOptions::new(Tolerance::METRE),),
        Err(GeomError::BackendContractViolation { backend, .. })
            if backend == BackendId::new("wrong-cardinality")
    ));
}

#[test]
fn adapter_runs_executor_policy_validation_before_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    let compiler = GpuCompiler::new(RejectingOptionsExecutor {
        inner: fake_executor("policy-rejection", true),
        compile_calls: calls.clone(),
    });
    let (graph, root) = point_graph(Vec3::ZERO);

    assert!(matches!(
        compiler.compile_mesh(&graph, root, &ExecutionOptions::new(Tolerance::METRE)),
        Err(GeomError::Unsupported { backend, .. })
            if backend == BackendId::new("policy-rejection")
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn adapter_rejects_incompatible_device_preferences() {
    let compiler = GpuCompiler::new(fake_executor("gpu-only", true));
    let (graph, root) = point_graph(Vec3::ZERO);
    let options = ExecutionOptions::new(Tolerance::METRE).with_device(DevicePreference::Cpu);

    assert!(matches!(
        compiler.compile_mesh(&graph, root, &options),
        Err(GeomError::Unsupported {
            backend,
            operation: Operation::GraphCompilation,
        }) if backend == BackendId::new("gpu-only")
    ));
}

fn fake_executor(id: &'static str, float64: bool) -> FakeExecutor {
    FakeExecutor {
        device: GpuDeviceDescriptor {
            id: BackendId::new(id),
            name: "test".to_owned(),
            api: "mock".to_owned(),
            features: GpuFeatures {
                float64,
                subgroups: false,
                unified_memory: true,
                max_workgroup_size: 64,
            },
        },
    }
}

fn point_graph(point: Vec3) -> (GeometryGraph, NodeId) {
    let mut builder = GeometryGraphBuilder::new();
    let root = builder.push(GeometryNode::Point3(point)).expect("root");
    (builder.finish(vec![root]).expect("graph"), root)
}

/// A device cannot deliver results into a *different* device's memory.
/// Catching that before dispatch is the point of tracking residency.
#[test]
fn results_wanted_on_a_foreign_device_are_refused_before_dispatch() {
    let compiler = GpuCompiler::new(fake_executor("home-gpu", true));
    let (graph, root) = point_graph(Vec3::ZERO);
    let options = ExecutionOptions::new(Tolerance::METRE).with_residency(DataResidency::new(
        Residency::Host,
        Residency::Device(BackendId::new("other-gpu")),
    ));

    assert!(matches!(
        compiler.compile_mesh(&graph, root, &options),
        Err(GeomError::Unsupported { backend, .. }) if backend == BackendId::new("home-gpu")
    ));
}

/// Results wanted on the executing device itself, or on the host, are fine.
#[test]
fn results_wanted_on_the_executing_device_or_host_are_accepted() {
    let compiler = GpuCompiler::new(fake_executor("home-gpu", true));
    let (graph, root) = point_graph(Vec3::ZERO);

    for output in [
        Residency::Host,
        Residency::Device(BackendId::new("home-gpu")),
        Residency::Unified(BackendId::new("home-gpu")),
    ] {
        let options = ExecutionOptions::new(Tolerance::METRE)
            .with_residency(DataResidency::new(Residency::Host, output));
        assert!(
            compiler.compile_mesh(&graph, root, &options).is_ok(),
            "{output:?} must be deliverable"
        );
    }
}

/// Both batch call shapes must reach the GPU in ONE submission. Overriding only
/// `compile_mesh_batch` would leave `compile_mesh_batch_into` silently falling back to a
/// per-root loop -- a real trap, since the fallback still returns correct
/// results while destroying the batching the seam exists for.
#[test]
fn both_batch_call_shapes_use_a_single_submission() {
    let submissions = Arc::new(AtomicUsize::new(0));
    let compiler = GpuCompiler::new(CountingExecutor {
        inner: fake_executor("counting-gpu", true),
        submissions: Arc::clone(&submissions),
    });
    let mut builder = GeometryGraphBuilder::new();
    let a = builder.push(GeometryNode::Point3(Vec3::ZERO)).expect("a");
    let b = builder.push(GeometryNode::Point3(Vec3::ZERO)).expect("b");
    let c = builder.push(GeometryNode::Point3(Vec3::ZERO)).expect("c");
    let graph = builder.finish(vec![a, b, c]).expect("graph");
    let options = ExecutionOptions::new(Tolerance::METRE);

    let owned = compiler
        .compile_mesh_batch(&graph, &[a, b, c], &options)
        .expect("batch");
    assert_eq!(owned.len(), 3);
    assert_eq!(submissions.load(Ordering::SeqCst), 1, "compile_mesh_batch");

    submissions.store(0, Ordering::SeqCst);
    let mut destination = Vec::new();
    compiler
        .compile_mesh_batch_into(&graph, &[a, b, c], &options, &mut destination)
        .expect("batch into");
    assert_eq!(destination.len(), 3);
    assert_eq!(
        submissions.load(Ordering::SeqCst),
        1,
        "compile_mesh_batch_into"
    );
}

/// `compile_mesh_batch_into` appends; it must not clear the caller's buffer.
#[test]
fn compile_mesh_batch_into_appends_rather_than_clearing() {
    let compiler = GpuCompiler::new(fake_executor("append-gpu", true));
    let (graph, root) = point_graph(Vec3::ZERO);
    let mut destination = vec![TriMesh::default()];
    compiler
        .compile_mesh_batch_into(
            &graph,
            &[root],
            &ExecutionOptions::new(Tolerance::METRE),
            &mut destination,
        )
        .expect("append");
    assert_eq!(destination.len(), 2, "existing entry must survive");
}

#[derive(Debug)]
struct CountingExecutor {
    inner: FakeExecutor,
    submissions: Arc<AtomicUsize>,
}

impl GpuGraphExecutor for CountingExecutor {
    fn device(&self) -> &GpuDeviceDescriptor {
        self.inner.device()
    }

    fn validate_options(&self, options: &ExecutionOptions) -> GeomResult<()> {
        self.inner.validate_options(options)
    }

    fn compile_mesh_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        self.submissions.fetch_add(1, Ordering::SeqCst);
        self.inner.compile_mesh_batch(graph, roots, options)
    }
}
