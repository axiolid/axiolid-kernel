//! Bounding an unbounded half-space against a finite boundary.
//!
//! A half-space is one side of a plane, infinite in every direction, so it
//! has no mesh of its own. To take part in a finite operation it must first
//! be bounded, and the boundary profile is what bounds it.
//!
//! The slab is built by extruding the boundary profile along the plane
//! normal, far enough in each direction to cover the boundary's own extent,
//! then keeping the selected side. Sizing from the boundary rather than
//! from a fixed constant is what keeps the result independent of model
//! units: a boundary in millimetres and the same boundary in metres
//! produce the same shape.

use axiolid_core::{Plane3, Point2, Scalar, Tolerance, Vec3};
use axiolid_kernel::{GeomError, GeomResult};
use axiolid_mesh::TriMesh;
use axiolid_primitive::{ClipMargin, HalfSpace};

use crate::loft::{self, Frame, Station};
use crate::profile::Rings;

/// Build the finite solid that represents a half-space bounded by a profile.
///
/// `agreement` selects which side of the plane survives: `true` keeps the
/// normal side, `false` the opposite one. The boundary profile is assumed
/// to lie in the plane's own frame, which is how the graph stores it.
///
/// The margin scales the extrusion depth relative to the boundary's own
/// size. It is relative rather than absolute so the result does not depend
/// on the model's units.
pub fn bounded_half_space(
    boundary: &Rings,
    plane: Plane3,
    agreement: bool,
    margin: ClipMargin,
    tolerance: Tolerance,
) -> GeomResult<TriMesh> {
    let normal = plane.normal.normalize_or_zero();
    if normal == Vec3::ZERO {
        return Err(GeomError::InvalidInput(
            "half-space boundary plane needs a non-zero normal".to_owned(),
        ));
    }
    if boundary.outer.len() < 3 {
        return Err(GeomError::InvalidInput(format!(
            "half-space boundary needs at least 3 vertices, got {}",
            boundary.outer.len()
        )));
    }

    // Size the slab from the boundary's own extent. A profile spanning 10
    // units gets a slab proportional to 10, so the construction carries no
    // hidden absolute length.
    let mut extent: Scalar = 0.0;
    for p in boundary.outer.iter().chain(boundary.holes.iter().flatten()) {
        extent = extent.max(p.x.abs()).max(p.y.abs());
    }
    // No zero-extent guard here: a boundary with no area cannot be
    // triangulated, so `loft` rejects it downstream with a message naming
    // the real problem. A guard here was unreachable, and unreachable
    // validation reads as a promise the code cannot keep.
    let depth = extent * margin.factor();

    // Frame the profile in the plane. The plane normal is the sweep
    // direction, so the profile's own x and y span the plane.
    let reference = if normal.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let base = Frame::from_reference(plane.origin, normal, reference)?;

    // Keeping the normal side sweeps from the plane along +normal; the
    // opposite side sweeps the other way.
    //
    // Reversing the sweep direction alone would mirror the frame, and a
    // mirrored frame inverts the handedness `loft` derived its winding
    // from: the solid comes out correct in shape but inside-out, with a
    // negative volume. Flipping one in-plane axis restores the handedness,
    // so `loft` stays the single source of truth for orientation.
    let step = if agreement { normal } else { -normal };
    let (x, y) = if agreement {
        (base.x, base.y)
    } else {
        (base.x, -base.y)
    };
    let near = Frame {
        origin: base.origin,
        x,
        y,
    };
    let far = Frame {
        origin: base.origin + step * depth,
        x,
        y,
    };
    let stations: Vec<Station> = [near, far]
        .iter()
        .map(|f| loft::place(boundary, |p| loft::at(f, p)))
        .collect();
    let _ = tolerance;
    loft::loft(boundary, &stations, false)
}

pub fn for_subject(
    subject: &TriMesh,
    half_space: HalfSpace,
    tolerance: Tolerance,
) -> GeomResult<TriMesh> {
    let normal = half_space.boundary.normal.normalize_or_zero();
    if normal == Vec3::ZERO || subject.positions.is_empty() {
        return Err(GeomError::InvalidInput(
            "half-space boolean needs finite subject bounds".into(),
        ));
    }
    let reference = if normal.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let frame = Frame::from_reference(half_space.boundary.origin, normal, reference)?;
    let mut min = Point2::splat(Scalar::INFINITY);
    let mut max = Point2::splat(Scalar::NEG_INFINITY);
    let side = if half_space.agreement { 1.0 } else { -1.0 };
    let mut selected_depth: Scalar = 0.0;
    for &point in &subject.positions {
        if !point.is_finite() {
            return Err(GeomError::InvalidInput(
                "half-space subject contains non-finite points".into(),
            ));
        }
        let delta = point - half_space.boundary.origin;
        let uv = Point2::new(delta.dot(frame.x), delta.dot(frame.y));
        min = min.min(uv);
        max = max.max(uv);
        selected_depth = selected_depth.max(delta.dot(normal) * side);
    }
    let span = (max - min).max_element().max(tolerance.linear());
    let pad = span * 0.1 + tolerance.linear();
    min -= Point2::splat(pad);
    max += Point2::splat(pad);
    let boundary = Rings {
        outer: vec![
            min,
            Point2::new(max.x, min.y),
            max,
            Point2::new(min.x, max.y),
        ],
        holes: Vec::new(),
    };
    let extent = min.abs().max(max.abs()).max_element();
    let depth = selected_depth + pad;
    let factor = depth / extent;
    let margin = ClipMargin::new(factor).ok_or_else(|| {
        GeomError::InvalidInput("half-space subject has no finite clipping extent".into())
    })?;
    bounded_half_space(
        &boundary,
        half_space.boundary,
        half_space.agreement,
        margin,
        tolerance,
    )
}
