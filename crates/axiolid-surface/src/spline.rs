//! Polynomial and rational B-spline surfaces.

use axiolid_core::{Point3, Scalar};
use axiolid_curve::KnotSpec;

/// Tensor-product B-spline surface preserving exact knot and weight data.
#[derive(Debug, Clone, PartialEq)]
pub struct BSplineSurface {
    /// Degree along the first parameter axis.
    pub u_degree: u16,
    /// Degree along the second parameter axis.
    pub v_degree: u16,
    /// Rectangular control net, row-major in `u` then `v`.
    pub control_points: Vec<Vec<Point3>>,
    /// Distinct knots along `u`.
    pub u_knots: Vec<Scalar>,
    /// Multiplicities matching `u_knots`.
    pub u_multiplicities: Vec<u32>,
    /// Distinct knots along `v`.
    pub v_knots: Vec<Scalar>,
    /// Multiplicities matching `v_knots`.
    pub v_multiplicities: Vec<u32>,
    /// Optional rational weight net matching the control net shape.
    pub weights: Option<Vec<Vec<Scalar>>>,
    /// Whether the surface closes along `u`.
    pub u_closed: bool,
    /// Whether the surface closes along `v`.
    pub v_closed: bool,
    /// Source knot convention.
    pub knot_spec: KnotSpec,
    /// Whether the source declares self intersection.
    pub self_intersect: Option<bool>,
}
