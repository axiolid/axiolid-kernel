#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! General NURBS algorithms over Axiolid's format-neutral B-spline values.
//!
//! This crate builds on the portable scalar oracle. It owns differential
//! geometry and exact shape-preserving transformations; importers and
//! tessellators are consumers, not the capability boundary.

mod axis;
mod certified_bezier;
mod certified_curve_distance;
mod certified_curve_intersection;
mod certified_curve_projection;
mod certified_projection;
mod certified_refinement;
mod curve_analysis;
mod curve_projection;
mod periodic;
mod projection;
mod surface_analysis;
mod surface_projection;
mod surface_transform;
mod transform;

pub use certified_curve_distance::{distance_curve2_certified, distance_curve3_certified};
pub use certified_curve_intersection::{
    intersect_curve2_certified, CertifiedCurveIntersection2, CurveIntersectionDegeneracy,
    TransverseCurveIntersection2,
};
pub use certified_curve_projection::{project_curve2_certified, project_curve3_certified};
pub use certified_projection::{
    CertifiedProjectionOptions, CurveDistanceCertificate2, CurveDistanceCertificate3,
    CurvePairParameterBox, CurveProjectionCertificate2, CurveProjectionCertificate3,
    ParameterInterval,
};
pub use curve_analysis::{analyze_curve2, analyze_curve3, CurveDifferential2, CurveDifferential3};
pub use curve_projection::{project_curve2, project_curve3};
pub use periodic::{
    curve2_seam_continuity, curve3_seam_continuity, wrap_curve2_parameter, wrap_curve3_parameter,
    SeamContinuity,
};
pub use projection::{
    CurveProjection2, CurveProjection3, ProjectionOptions, ProjectionStatus, SurfaceProjection,
};
pub use surface_analysis::{analyze_surface, FundamentalForm, SurfaceDifferential};
pub use surface_projection::project_surface;
pub use surface_transform::{
    insert_surface_knot_u, insert_surface_knot_v, reverse_surface_u, reverse_surface_v,
};
pub use transform::{
    bezier_segments2, bezier_segments3, insert_knot2, insert_knot3, reverse2, reverse3, split2,
    split3,
};
