//! Closed set of format-neutral graph node families.

use axiolid_core::{Aabb, Frame2, Frame3, Point2, Point3, Scalar, Transform3, Vec2, Vec3};
use axiolid_curve::{Curve2, Curve3};
use axiolid_mesh::{PolygonMesh, TriMesh};
use axiolid_primitive::{HalfSpace, Primitive};
use axiolid_profile::Profile;
use axiolid_surface::Surface;
use axiolid_topology::BRep;

use crate::{CurveRelation, NodeId, SolidOperation, SurfaceRelation};

/// Point constrained to a curve parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointOnCurve {
    /// Basis curve.
    pub curve: NodeId,
    /// Curve parameter.
    pub parameter: Scalar,
}

/// Point constrained to surface parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointOnSurface {
    /// Basis surface.
    pub surface: NodeId,
    /// First parameter.
    pub u: Scalar,
    /// Second parameter.
    pub v: Scalar,
}

/// Reuse one graph node under a transform, preserving instancing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instance {
    /// Reused source node.
    pub source: NodeId,
    /// Local-to-parent transform.
    pub transform: Transform3,
}

/// One node in an immutable geometry DAG.
///
/// The enum is non-exhaustive so representation growth is semver-compatible.
/// Compilers must turn unknown/unsupported families into a structured
/// `Unsupported` result rather than a wildcard no-op.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryNode {
    /// Two-dimensional position.
    Point2(Point2),
    /// Three-dimensional position.
    Point3(Point3),
    /// Two-dimensional direction/vector.
    Vector2(Vec2),
    /// Three-dimensional direction/vector.
    Vector3(Vec3),
    /// Two-dimensional axis placement.
    Frame2(Frame2),
    /// Three-dimensional axis placement.
    Frame3(Frame3),
    /// Composed affine transform.
    Transform(Transform3),
    /// Dense two-dimensional point list.
    PointList2(Vec<Point2>),
    /// Dense three-dimensional point list.
    PointList3(Vec<Point3>),
    /// Atomic two-dimensional curve.
    Curve2(Curve2),
    /// Atomic three-dimensional curve.
    Curve3(Curve3),
    /// Curve composition/constraint.
    CurveRelation(CurveRelation),
    /// Point on a curve.
    PointOnCurve(PointOnCurve),
    /// Atomic surface.
    Surface(Surface),
    /// Surface composition/constraint.
    SurfaceRelation(SurfaceRelation),
    /// Point on a surface.
    PointOnSurface(PointOnSurface),
    /// Exact section profile.
    Profile(Profile),
    /// Exact primitive solid.
    Primitive(Primitive),
    /// Unbounded half-space.
    HalfSpace(HalfSpace),
    /// Solid construction instruction.
    SolidOperation(SolidOperation),
    /// Exact topological representation linked to graph curve/surface nodes.
    BRep(BRep<NodeId>),
    /// Polygonal n-gon mesh before triangulation.
    PolygonMesh(PolygonMesh),
    /// Triangle mesh.
    TriMesh(TriMesh),
    /// Axis-aligned bounds.
    BoundingBox(Aabb),
    /// Instanced/mapped geometry.
    Instance(Instance),
    /// Ordered geometric set or representation.
    Collection(Vec<NodeId>),
}

impl GeometryNode {
    /// Direct graph dependencies in deterministic order.
    pub fn references(&self) -> Vec<NodeId> {
        let mut references = Vec::new();
        match self {
            Self::CurveRelation(value) => value.references(&mut references),
            Self::PointOnCurve(value) => references.push(value.curve),
            Self::SurfaceRelation(value) => value.references(&mut references),
            Self::PointOnSurface(value) => references.push(value.surface),
            Self::SolidOperation(value) => value.references(&mut references),
            Self::BRep(value) => {
                references.extend(value.edges().iter().filter_map(|edge| edge.curve));
                references.extend(value.faces().iter().filter_map(|face| face.surface));
            }
            Self::Instance(value) => references.push(value.source),
            Self::Collection(values) => references.extend(values.iter().copied()),
            Self::Point2(_)
            | Self::Point3(_)
            | Self::Vector2(_)
            | Self::Vector3(_)
            | Self::Frame2(_)
            | Self::Frame3(_)
            | Self::Transform(_)
            | Self::PointList2(_)
            | Self::PointList3(_)
            | Self::Curve2(_)
            | Self::Curve3(_)
            | Self::Surface(_)
            | Self::Profile(_)
            | Self::Primitive(_)
            | Self::HalfSpace(_)
            | Self::PolygonMesh(_)
            | Self::TriMesh(_)
            | Self::BoundingBox(_) => {}
        }
        references
    }
}
