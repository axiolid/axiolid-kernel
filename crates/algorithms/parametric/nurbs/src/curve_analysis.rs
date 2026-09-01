//! Differential properties of regular parametric curves.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point2, Point3, Scalar, Tolerance, Vec2, Vec3};
use axiolid_curve::{Curve2, Curve3};
use axiolid_reference::curve::{jet2, jet3};

/// Parameter-invariant differential properties of a planar curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveDifferential2 {
    /// Evaluated point.
    pub point: Point2,
    /// Unit tangent in increasing-parameter direction.
    pub unit_tangent: Vec2,
    /// Unsigned parameter-invariant curvature.
    pub curvature: Scalar,
    /// Signed planar curvature; positive bends toward the tangent's left normal.
    pub signed_curvature: Scalar,
    /// Left unit normal scaled by signed curvature.
    pub curvature_vector: Vec2,
}

/// Parameter-invariant differential properties of a spatial curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveDifferential3 {
    /// Evaluated point.
    pub point: Point3,
    /// Unit tangent in increasing-parameter direction.
    pub unit_tangent: Vec3,
    /// Unsigned parameter-invariant curvature.
    pub curvature: Scalar,
    /// Normal-acceleration direction scaled by curvature.
    pub curvature_vector: Vec3,
}

/// Analyze a regular planar curve at `t`.
pub fn analyze_curve2(
    curve: &Curve2,
    t: Scalar,
    tolerance: Tolerance,
) -> GeomResult<CurveDifferential2> {
    reject_polyline_corner2(curve, t)?;
    let j = jet2(curve, t)?;
    let speed = j.first.length();
    regular(speed, tolerance)?;
    let signed = j.first.perp_dot(j.second) / speed.powi(3);
    let tangent = j.first / speed;
    let normal = Vec2::new(-tangent.y, tangent.x);
    finite(signed)?;
    Ok(CurveDifferential2 {
        point: j.point,
        unit_tangent: tangent,
        curvature: signed.abs(),
        signed_curvature: signed,
        curvature_vector: normal * signed,
    })
}

/// Analyze a regular spatial curve at `t`.
pub fn analyze_curve3(
    curve: &Curve3,
    t: Scalar,
    tolerance: Tolerance,
) -> GeomResult<CurveDifferential3> {
    reject_polyline_corner3(curve, t)?;
    let j = jet3(curve, t)?;
    let speed = j.first.length();
    regular(speed, tolerance)?;
    let tangent = j.first / speed;
    let normal_acceleration = j.second - tangent * j.second.dot(tangent);
    let curvature_vector = normal_acceleration / speed.powi(2);
    let curvature = curvature_vector.length();
    finite(curvature)?;
    Ok(CurveDifferential3 {
        point: j.point,
        unit_tangent: tangent,
        curvature,
        curvature_vector,
    })
}

fn reject_polyline_corner2(curve: &Curve2, t: Scalar) -> GeomResult<()> {
    if let Curve2::Polyline(polyline) = curve {
        reject_corner(polyline.points.len(), polyline.closed, t)?;
    }
    Ok(())
}

fn reject_polyline_corner3(curve: &Curve3, t: Scalar) -> GeomResult<()> {
    if let Curve3::Polyline(polyline) = curve {
        reject_corner(polyline.points.len(), polyline.closed, t)?;
    }
    Ok(())
}

fn reject_corner(point_count: usize, closed: bool, t: Scalar) -> GeomResult<()> {
    if !t.is_finite() {
        return Ok(());
    }
    let segment_count = if closed {
        point_count
    } else {
        point_count.saturating_sub(1)
    };
    if segment_count == 0 {
        return Ok(());
    }
    let end = segment_count as Scalar;
    let parameter = t.clamp(0.0, end);
    let is_vertex = parameter.fract() == 0.0;
    let is_two_sided = closed || (parameter > 0.0 && parameter < end);
    if is_vertex && is_two_sided {
        Err(GeomError::Degenerate(
            "polyline curvature is undefined at a non-smooth vertex".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn regular(speed: Scalar, tolerance: Tolerance) -> GeomResult<()> {
    if !speed.is_finite() || speed <= tolerance.linear() {
        Err(GeomError::Degenerate(format!(
            "curve speed {speed} does not exceed the linear tolerance"
        )))
    } else {
        Ok(())
    }
}

fn finite(value: Scalar) -> GeomResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(GeomError::Degenerate(
            "curve differential is non-finite".to_owned(),
        ))
    }
}
