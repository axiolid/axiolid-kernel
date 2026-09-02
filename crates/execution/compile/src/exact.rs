//! Reference graph-to-exact-B-rep compiler.
//!
//! Exact and mesh compilation are separate result domains. One exact batch owns
//! a `NodeId -> ExactBRep` memo table; a discrete value cannot enter that cache.

use std::collections::{HashMap, HashSet};

use axiolid_brep::ExactBRep;
use axiolid_construct::extrude::extrude_profile_exact;
use axiolid_contracts::{
    Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomError,
    GeomResult, Operation,
};
use axiolid_exact_compile_contract::ExactCompiler;
use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};

/// Scalar reference implementation of the exact-compilation capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReferenceExactCompiler;

impl ReferenceExactCompiler {
    /// Stable identity of this backend.
    pub const ID: BackendId = BackendId::new("scalar-exact-compile");

    /// Construct the reference exact compiler.
    pub const fn new() -> Self {
        Self
    }
}

impl Backend for ReferenceExactCompiler {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(Self::ID, ExecutionTarget::PortableCpu)
    }
}

impl ExactCompiler for ReferenceExactCompiler {
    fn compile_exact(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<ExactBRep> {
        ExactCompilation::new(graph, options).compile(root)
    }

    fn compile_exact_batch_into(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
        destination: &mut Vec<ExactBRep>,
    ) -> GeomResult<()> {
        destination.reserve(roots.len());
        let mut compilation = ExactCompilation::new(graph, options);
        for &root in roots {
            destination.push(compilation.compile(root)?);
        }
        Ok(())
    }
}

struct ExactCompilation<'a> {
    graph: &'a GeometryGraph,
    options: &'a ExecutionOptions,
    cache: HashMap<NodeId, ExactBRep>,
    active: HashSet<NodeId>,
    cache_hits: usize,
    evaluated_nodes: usize,
}

impl<'a> ExactCompilation<'a> {
    fn new(graph: &'a GeometryGraph, options: &'a ExecutionOptions) -> Self {
        Self {
            graph,
            options,
            cache: HashMap::new(),
            active: HashSet::new(),
            cache_hits: 0,
            evaluated_nodes: 0,
        }
    }

    fn compile(&mut self, root: NodeId) -> GeomResult<ExactBRep> {
        if let Some(cached) = self.cache.get(&root) {
            self.cache_hits += 1;
            return Ok(cached.clone());
        }
        if !self.active.insert(root) {
            return Err(GeomError::InvalidInput(format!(
                "exact compilation cycle reached at node {root:?}"
            )));
        }

        self.evaluated_nodes += 1;
        let result = self.compile_uncached(root);
        self.active.remove(&root);
        let exact = result?;
        self.cache.insert(root, exact.clone());
        Ok(exact)
    }

    fn compile_uncached(&self, root: NodeId) -> GeomResult<ExactBRep> {
        let node = self.graph.get(root).ok_or_else(|| {
            GeomError::InvalidInput(format!("node {root:?} does not belong to this graph"))
        })?;

        match node {
            GeometryNode::SolidOperation(SolidOperation::Extrusion {
                profile,
                direction,
                depth,
            }) => {
                let profile_node = self.graph.get(*profile).ok_or_else(|| {
                    GeomError::InvalidInput(format!(
                        "extrusion profile {profile:?} does not belong to this graph"
                    ))
                })?;
                let GeometryNode::Profile(profile) = profile_node else {
                    return Err(unsupported("extrusion profile reference"));
                };
                extrude_profile_exact(profile, *direction, *depth, self.options.tolerance())
            }
            _ => Err(unsupported(exact_input_family(node))),
        }
    }
}

fn unsupported(input: &'static str) -> GeomError {
    GeomError::UnsupportedInput {
        backend: ReferenceExactCompiler::ID,
        operation: Operation::GraphCompilation,
        input,
    }
}

fn exact_input_family(node: &GeometryNode) -> &'static str {
    match node {
        GeometryNode::Point2(_) => "2D point",
        GeometryNode::Point3(_) => "3D point",
        GeometryNode::Vector2(_) => "2D vector",
        GeometryNode::Vector3(_) => "3D vector",
        GeometryNode::Frame2(_) => "2D frame",
        GeometryNode::Frame3(_) => "3D frame",
        GeometryNode::Transform(_) => "transform",
        GeometryNode::PointList2(_) => "2D point list",
        GeometryNode::PointList3(_) => "3D point list",
        GeometryNode::Primitive(_) => "primitive",
        GeometryNode::Curve2(_) => "2D curve",
        GeometryNode::Curve3(_) => "3D curve",
        GeometryNode::CurveRelation(_) => "curve relation",
        GeometryNode::PointOnCurve(_) => "point on curve",
        GeometryNode::Surface(_) => "surface",
        GeometryNode::SurfaceRelation(_) => "surface relation",
        GeometryNode::PointOnSurface(_) => "point on surface",
        GeometryNode::Profile(_) => "profile",
        GeometryNode::OpenProfile(_) => "open profile",
        GeometryNode::HalfSpace(_) => "half-space",
        GeometryNode::BRep(_) => "B-rep",
        GeometryNode::PolygonMesh(_) => "polygon mesh",
        GeometryNode::TriMesh(_) => "triangle mesh",
        GeometryNode::BoundingBox(_) => "bounding box",
        GeometryNode::SolidOperation(operation) => solid_operation_family(operation),
        GeometryNode::Instance(_) => "instance",
        GeometryNode::Collection(_) => "collection",
        _ => "unknown geometry node",
    }
}

fn solid_operation_family(operation: &SolidOperation) -> &'static str {
    match operation {
        SolidOperation::Extrusion { .. } => "extrusion",
        SolidOperation::TaperedExtrusion { .. } => "tapered extrusion",
        SolidOperation::Revolution { .. } => "revolution",
        SolidOperation::TaperedRevolution { .. } => "tapered revolution",
        SolidOperation::SweptDisk { .. } => "swept disk",
        SolidOperation::FixedReferenceSweep { .. } => "fixed-reference sweep",
        SolidOperation::SurfaceCurveSweep { .. } => "surface-curve sweep",
        SolidOperation::SectionedSpine { .. } => "sectioned spine",
        SolidOperation::Boolean { .. } => "boolean",
        SolidOperation::BoundedHalfSpace { .. } => "bounded half-space",
        _ => "unknown solid operation",
    }
}

#[cfg(test)]
mod tests {
    use axiolid_core::{Tolerance, Vec3};
    use axiolid_model::GeometryGraphBuilder;
    use axiolid_profile::{Profile, RectangleProfile};

    use super::*;

    #[test]
    fn duplicate_roots_hit_the_exact_result_cache() {
        let mut builder = GeometryGraphBuilder::new();
        let profile = builder
            .push(GeometryNode::Profile(Profile::Rectangle(
                RectangleProfile {
                    x: 2.0,
                    y: 1.0,
                    thickness: None,
                    outer_radius: None,
                    inner_radius: None,
                },
            )))
            .unwrap();
        let root = builder
            .push(GeometryNode::SolidOperation(SolidOperation::Extrusion {
                profile,
                direction: Vec3::Z,
                depth: 1.0,
            }))
            .unwrap();
        let graph = builder.finish(vec![root]).unwrap();
        let options = ExecutionOptions::new(Tolerance::METRE);
        let mut compilation = ExactCompilation::new(&graph, &options);

        let first = compilation.compile(root).unwrap();
        let second = compilation.compile(root).unwrap();

        assert_eq!(first, second);
        assert_eq!(compilation.evaluated_nodes, 1);
        assert_eq!(compilation.cache_hits, 1);
        assert_eq!(compilation.cache.len(), 1);
    }
}
