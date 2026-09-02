//! Reference graph-to-exact-B-rep compiler.
//!
//! This is the exact counterpart to [`crate::ReferenceMeshCompiler`]. It
//! advertises `GRAPH_TO_EXACT_BREP` and refuses unsupported node families with
//! an input-specific diagnostic. It never delegates to the mesh compiler and
//! never returns an approximation for an exact request.

use axiolid_brep::ExactBRep;
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
        _options: &ExecutionOptions,
    ) -> GeomResult<ExactBRep> {
        let node = graph.get(root).ok_or_else(|| {
            GeomError::InvalidInput(format!("node {root:?} does not belong to this graph"))
        })?;
        Err(GeomError::UnsupportedInput {
            backend: Self::ID,
            operation: Operation::GraphCompilation,
            input: exact_input_family(node),
        })
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
