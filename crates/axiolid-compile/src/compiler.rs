//! The `GeometryCompiler` implementation: graph in, meshes out.
//!
//! Evaluation is iterative. The graph forbids non-prior references,
//! so cycles are structurally impossible, but depth is unbounded and
//! recursion would risk a stack overflow on adversarial input.

use axiolid_core::{Scalar, Transform3};
use axiolid_kernel::{
    Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomError,
    GeomResult, GeometryCompiler, MeshBoolean, Operation, ScratchRequirement,
};
use axiolid_mesh::TriMesh;
use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};

use crate::extrude::extrude_profile;
use crate::profile::profile_rings;

/// Scalar reference compiler.
///
/// Generic over the boolean provider so this crate never depends on a
/// particular one: `axiolid-boolmesh` is an adapter, and a different provider
/// swaps in without touching this code.
#[derive(Debug, Clone)]
pub struct ScalarCompiler<B> {
    boolean: B,
}

impl<B> ScalarCompiler<B> {
    /// Bind a boolean provider.
    pub const fn new(boolean: B) -> Self {
        Self { boolean }
    }

    /// The bound provider.
    pub const fn boolean(&self) -> &B {
        &self.boolean
    }
}

impl<B: MeshBoolean> Backend for ScalarCompiler<B> {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(
            BackendId::new("scalar-compile"),
            ExecutionTarget::PortableCpu,
        )
    }
}

/// One entry in the explicit evaluation stack.
///
/// `Enter` schedules dependency discovery; `Exit` runs after every dependency
/// already has a mesh, which is what replaces the recursive call.
#[derive(Debug, Clone, Copy)]
enum Step {
    Enter(NodeId),
    Exit(NodeId),
}

/// Cached meshes keyed by node index.
///
/// A DAG can reference one node from several parents; without memoisation a
/// shared subtree is recompiled once per reference, which is exponential on
/// deeply shared graphs.
type Cache = std::collections::HashMap<usize, TriMesh>;

impl<B: MeshBoolean> ScalarCompiler<B> {
    /// Resolve a node handle, blaming the graph rather than panicking.
    fn node<'g>(&self, graph: &'g GeometryGraph, id: NodeId) -> GeomResult<&'g GeometryNode> {
        graph.get(id).ok_or_else(|| {
            GeomError::InvalidInput(format!("node {id:?} does not belong to this graph"))
        })
    }

    /// Nodes that must have meshes before `id` can be evaluated.
    ///
    /// Only mesh-producing dependencies are listed. A profile referenced by an
    /// extrusion is consumed as 2D data, not as a mesh, so it is deliberately
    /// absent: compiling it standalone would be meaningless.
    fn mesh_dependencies(&self, node: &GeometryNode) -> Vec<NodeId> {
        match node {
            GeometryNode::Instance(instance) => vec![instance.source],
            GeometryNode::Collection(members) => members.clone(),
            GeometryNode::SolidOperation(SolidOperation::Boolean { left, right, .. }) => {
                vec![*left, *right]
            }
            _ => Vec::new(),
        }
    }

    /// Iterative post-order evaluation of one root.
    fn evaluate(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
        cache: &mut Cache,
    ) -> GeomResult<TriMesh> {
        let mut stack = vec![Step::Enter(root)];
        while let Some(step) = stack.pop() {
            match step {
                Step::Enter(id) => {
                    if cache.contains_key(&id.index()) {
                        continue;
                    }
                    let node = self.node(graph, id)?;
                    let deps = self.mesh_dependencies(node);
                    // Exit runs after every dependency, so push it first.
                    stack.push(Step::Exit(id));
                    for dep in deps {
                        if !cache.contains_key(&dep.index()) {
                            stack.push(Step::Enter(dep));
                        }
                    }
                }
                Step::Exit(id) => {
                    if cache.contains_key(&id.index()) {
                        continue;
                    }
                    let mesh = self.build(graph, id, options, cache)?;
                    cache.insert(id.index(), mesh);
                }
            }
        }
        cache
            .get(&root.index())
            .cloned()
            .ok_or_else(|| GeomError::InvalidInput(format!("root {root:?} produced no mesh")))
    }

    /// Build one node, assuming its mesh dependencies are already cached.
    fn build(
        &self,
        graph: &GeometryGraph,
        id: NodeId,
        options: &ExecutionOptions,
        cache: &Cache,
    ) -> GeomResult<TriMesh> {
        let node = self.node(graph, id)?;
        match node {
            GeometryNode::TriMesh(mesh) => Ok(mesh.clone()),
            GeometryNode::Instance(instance) => {
                let source = self.cached(cache, instance.source)?;
                Ok(transform_mesh(source, instance.transform))
            }
            GeometryNode::Collection(members) => {
                let mut merged = TriMesh::default();
                for &member in members {
                    append_mesh(&mut merged, self.cached(cache, member)?);
                }
                Ok(merged)
            }
            GeometryNode::SolidOperation(operation) => {
                self.build_solid(graph, operation, options, cache)
            }
            GeometryNode::BRep(brep) => crate::brep::tessellate(brep, graph),
            other => Err(GeomError::Unsupported {
                backend: self.descriptor().id,
                operation: unsupported_operation(other),
            }),
        }
    }

    /// Read an already-built dependency.
    fn cached<'c>(&self, cache: &'c Cache, id: NodeId) -> GeomResult<&'c TriMesh> {
        cache.get(&id.index()).ok_or_else(|| {
            GeomError::InvalidInput(format!("dependency {id:?} was not evaluated first"))
        })
    }

    /// Extrusion and boolean; every other solid family is explicitly refused.
    fn build_solid(
        &self,
        graph: &GeometryGraph,
        operation: &SolidOperation,
        options: &ExecutionOptions,
        cache: &Cache,
    ) -> GeomResult<TriMesh> {
        match operation {
            SolidOperation::Extrusion {
                profile,
                direction,
                depth,
            } => {
                let node = self.node(graph, *profile)?;
                let GeometryNode::Profile(shape) = node else {
                    return Err(GeomError::InvalidInput(format!(
                        "extrusion profile {profile:?} is not a Profile node"
                    )));
                };
                let rings = profile_rings(shape, chord_error(options), options.tolerance())?;
                extrude_profile(&rings, *direction, *depth, options.tolerance())
            }
            SolidOperation::Boolean {
                left,
                right,
                operator,
            } => {
                let subject = self.cached(cache, *left)?;
                let tool = self.cached(cache, *right)?;
                // The compiler produces a mesh graph, so it takes the mesh and
                // drops the evidence here. A caller wanting boolean diagnostics
                // calls the registry directly; threading evidence through every
                // DAG node would change the compile contract, which is a
                // separate decision from fixing the boolean contract.
                self.boolean
                    .boolean(subject, tool, *operator, options)
                    .map(|outcome| outcome.mesh)
            }
            // Every remaining family is a sweep of some kind (revolution,
            // swept disk, fixed-reference, surface-curve, sectioned spine) or
            // a bounded half-space. Naming the capability lets a caller
            // register a provider for it rather than guess.
            _ => Err(GeomError::Unsupported {
                backend: self.descriptor().id,
                operation: Operation::Sweep,
            }),
        }
    }
}

/// Chord budget for curve flattening.
///
/// Derived from the operation tolerance rather than a global constant: the
/// acceptable sagitta is a property of the model's unit scale.
fn chord_error(options: &ExecutionOptions) -> Scalar {
    options.tolerance().linear()
}

/// Which capability a node family would need.
///
/// Reporting the real missing capability lets a caller register a provider
/// that supplies it, instead of guessing from a generic failure.
fn unsupported_operation(node: &GeometryNode) -> Operation {
    match node {
        GeometryNode::Curve2(_) | GeometryNode::Curve3(_) => Operation::CurveEvaluation,
        GeometryNode::Surface(_) => Operation::SurfaceEvaluation,
        GeometryNode::Profile(_) => Operation::ProfileTriangulation,
        GeometryNode::BRep(_) | GeometryNode::PolygonMesh(_) => Operation::Tessellation,
        _ => Operation::GraphCompilation,
    }
}

/// Apply an affine transform to every position.
///
/// Normals are dropped rather than transformed: a correct normal transform is
/// the inverse transpose, and silently applying the point transform would
/// produce subtly wrong shading under non-uniform scale.
fn transform_mesh(mesh: &TriMesh, transform: Transform3) -> TriMesh {
    let positions = mesh
        .positions
        .iter()
        .map(|&p| transform.transform_point3(p))
        .collect();
    let mut out = TriMesh::new(positions, mesh.indices.clone());
    if transform.matrix3.determinant() < 0.0 {
        // A mirroring transform reverses orientation; restore outward winding
        // so the result still satisfies the boolean provider's precondition.
        for triangle in out.indices.chunks_exact_mut(3) {
            triangle.swap(1, 2);
        }
    }
    out
}

/// Concatenate `source` into `target`, rebasing indices.
fn append_mesh(target: &mut TriMesh, source: &TriMesh) {
    let offset = target.positions.len() as u32;
    target.positions.extend_from_slice(&source.positions);
    target
        .indices
        .extend(source.indices.iter().map(|&i| i + offset));
}

impl<B: MeshBoolean> GeometryCompiler for ScalarCompiler<B> {
    /// Bounded by the peak mesh size, which is data-dependent, so the honest
    /// answer is unbounded rather than an invented constant.
    fn scratch_requirement(&self) -> ScratchRequirement {
        ScratchRequirement::Unbounded
    }

    fn compile(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let mut cache = Cache::new();
        self.evaluate(graph, root, options, &mut cache)
    }

    /// Overriding the `_into` seam gives both call shapes one shared cache,
    /// so a subtree referenced by several roots is compiled once per batch.
    fn compile_batch_into(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
        destination: &mut Vec<TriMesh>,
    ) -> GeomResult<()> {
        destination.reserve(roots.len());
        let mut cache = Cache::new();
        for &root in roots {
            destination.push(self.evaluate(graph, root, options, &mut cache)?);
        }
        Ok(())
    }
}

impl<B: MeshBoolean> ScalarCompiler<B> {
    /// The boolean provider this compiler dispatches to.
    ///
    /// Exposed so an application can apply its own source-format set
    /// operations with the same provider the compiler uses, rather than
    /// constructing a second one that might differ.
    pub const fn boolean_provider(&self) -> &B {
        &self.boolean
    }
}
