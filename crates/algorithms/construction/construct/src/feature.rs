//! Chamfer and fillet on a straight vertical edge of an exact prism (#68).
//!
//! # The contract
//!
//! An edge is selected by the CORNER it belongs to, not by an opaque index:
//! a caller naming "the corner nearest this point" survives a topology change
//! that renumbers edges, while an index does not.
//!
//! # What is exact here
//!
//! Chamfering a vertical edge of a prism replaces one corner of its profile
//! with a straight cut, so the result is a prism over a polygon with one more
//! vertex. That is exactly the shape `extrude_polygon_rings` already builds --
//! no new surface families, no approximation.
//!
//! A constant-radius FILLET replaces the corner with a circular arc, so the
//! wall becomes a cylindrical face. That is representable, but stitching a
//! cylindrical wall into the prism assembly is not the same construction, so
//! it is refused here rather than approximated by a polyline. Refusing is the
//! honest answer: a caller asking for a fillet and receiving a many-segment
//! chamfer would have no way to tell.
//!
//! # Refusals
//!
//! Variable radius, edge loops, curved edges, and fillets all return typed
//! refusals naming the missing capability, never a silent no-op. A no-op is
//! the worst outcome: the caller believes the feature was applied.

use axiolid_brep::ExactBRep;
use axiolid_contracts::{GeomError, GeomResult, Operation};
use axiolid_core::{Point2, Scalar, Tolerance, Vec3};
use axiolid_profile::{Profile, RectangleProfile};

use crate::extrude_exact::extrude_polygon_rings;
use crate::BACKEND_ID;

fn unsupported(input: &'static str) -> GeomError {
    GeomError::UnsupportedInput {
        backend: BACKEND_ID,
        operation: Operation::Sweep,
        input,
    }
}

/// Which edge a feature applies to.
///
/// Selecting by position rather than index keeps a caller's request stable
/// across a topology change that renumbers edges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeSelector {
    /// The vertical edge at the profile corner nearest this point.
    NearestCorner(Point2),
}

/// How much material a feature removes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FeatureSize {
    /// Constant setback measured along both faces meeting at the edge.
    ConstantDistance(Scalar),
    /// Constant blend radius. Representable, not yet constructible here.
    ConstantRadius(Scalar),
}

/// Chamfer one vertical edge of an exact extruded rectangle.
///
/// Supported: a sharp filled rectangle extruded along +z, chamfered at one
/// corner by a constant distance. Everything else is refused by name.
pub fn chamfer_extruded_profile(
    profile: &Profile,
    direction: Vec3,
    depth: Scalar,
    edge: EdgeSelector,
    size: FeatureSize,
    tolerance: Tolerance,
) -> GeomResult<ExactBRep> {
    let distance = match size {
        FeatureSize::ConstantDistance(distance) => distance,
        // A fillet needs a cylindrical wall stitched into the prism assembly,
        // which is a different construction. Approximating it with a
        // multi-segment chamfer would be indistinguishable to the caller.
        FeatureSize::ConstantRadius(_) => {
            return Err(unsupported("constant-radius fillet on an extruded solid"))
        }
    };
    if !distance.is_finite() || distance <= 0.0 {
        return Err(GeomError::InvalidInput(format!(
            "chamfer distance must be positive and finite, got {distance}"
        )));
    }

    let Profile::Rectangle(rectangle) = profile else {
        return Err(unsupported("chamfer on a non-rectangle profile"));
    };
    let RectangleProfile {
        x,
        y,
        thickness,
        outer_radius,
        inner_radius,
    } = *rectangle;
    if thickness.is_some() {
        return Err(unsupported("chamfer on a hollow profile"));
    }
    if outer_radius.is_some() || inner_radius.is_some() {
        return Err(unsupported("chamfer on an already-rounded profile"));
    }
    if !x.is_finite() || !y.is_finite() || x <= 0.0 || y <= 0.0 {
        return Err(GeomError::InvalidInput(format!(
            "chamfer profile must have positive finite extents, got {x} x {y}"
        )));
    }
    if !depth.is_finite() || depth <= 0.0 {
        return Err(GeomError::InvalidInput(format!(
            "chamfer extrusion depth must be positive and finite, got {depth}"
        )));
    }
    // Only a +z extrusion keeps the vertical edges vertical, which is what
    // makes the chamfered result another prism.
    if direction.normalize_or_zero().dot(Vec3::Z) < 1.0 - tolerance.linear() {
        return Err(unsupported("chamfer on an oblique extrusion"));
    }

    let (half_x, half_y) = (x / 2.0, y / 2.0);
    // Counter-clockwise, matching the outer ring extrude_rectangle builds.
    let corners = [
        Point2::new(-half_x, -half_y),
        Point2::new(half_x, -half_y),
        Point2::new(half_x, half_y),
        Point2::new(-half_x, half_y),
    ];

    // The setback is measured along each adjacent edge, so it cannot reach
    // the neighbouring corner: at exactly half an edge the two chamfers meet
    // and the edge vanishes, which is a different topology.
    if distance * 2.0 >= x || distance * 2.0 >= y {
        return Err(GeomError::Degenerate(format!(
            "chamfer distance {distance} consumes an entire edge of the {x} x {y} profile"
        )));
    }

    let EdgeSelector::NearestCorner(target) = edge;
    let index = corners
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (**a - target)
                .length_squared()
                .total_cmp(&(**b - target).length_squared())
        })
        .map(|(index, _)| index)
        .expect("the corner list is never empty");

    // Replace the corner with two points, each set back along one adjacent
    // edge. The corner's own vertex disappears: that is the chamfer.
    let previous = corners[(index + corners.len() - 1) % corners.len()];
    let corner = corners[index];
    let next = corners[(index + 1) % corners.len()];
    let into_previous = (previous - corner).normalize();
    let into_next = (next - corner).normalize();

    let mut ring = Vec::with_capacity(corners.len() + 1);
    for (position, point) in corners.iter().enumerate() {
        if position == index {
            // Order matters: entering along the previous edge, leaving along
            // the next one, so the ring keeps its counter-clockwise winding.
            ring.push(corner + into_previous * distance);
            ring.push(corner + into_next * distance);
        } else {
            ring.push(*point);
        }
    }

    extrude_polygon_rings(&[ring], Vec3::Z * depth)
}
