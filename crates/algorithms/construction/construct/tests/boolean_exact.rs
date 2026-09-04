//! Exact boolean over coaxial prisms, differentially tested (#66).
//!
//! Exact boolean previously reached only half-space-bounded difference and
//! intersection. Two prisms sharing an axis are the family where the general
//! polyhedral problem collapses to one the kernel already solves exactly: the
//! 2D boolean of their cross-sections crossed with their height intervals.
//!
//! The oracle is the cross-section area, derived from the input rectangles by
//! hand rather than recorded from a run. A wall with a rectangular opening is
//! exactly this shape, so this is the dominant building-model pattern, not a
//! toy case.

use axiolid_construct::boolean_exact::{boolean_prisms_exact, Prism};
use axiolid_contracts::GeomError;
use axiolid_core::{BooleanOperator, Point2, Tolerance};

/// An axis-aligned rectangle ring, counter-clockwise.
fn rect_ring(cx: f64, cy: f64, w: f64, h: f64) -> Vec<Point2> {
    let (hw, hh) = (w / 2.0, h / 2.0);
    vec![
        Point2::new(cx - hw, cy - hh),
        Point2::new(cx + hw, cy - hh),
        Point2::new(cx + hw, cy + hh),
        Point2::new(cx - hw, cy + hh),
    ]
}

fn prism(rings: Vec<Vec<Point2>>, bottom: f64, top: f64) -> Prism {
    Prism { rings, bottom, top }
}

/// Axis-aligned extent of the built solid's base cross-section.
///
/// Deliberately NOT a shoelace over all base vertices: with a hole present
/// those vertices belong to two separate rings, and treating them as one
/// polygon produces a meaningless number. The bounding extent is enough to
/// show the outer boundary was preserved, and the hole is checked separately
/// through the cap's loop count.
fn base_extent(brep: &axiolid_brep::ExactBRep) -> (f64, f64) {
    let points: Vec<(f64, f64)> = brep
        .topology()
        .vertices()
        .iter()
        .filter(|v| v.position.z.abs() < 1e-9)
        .map(|v| (v.position.x, v.position.y))
        .collect();
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for (x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    (max_x - min_x, max_y - min_y)
}

/// Shoelace area over a simple (hole-free) base cross-section.
fn base_area(brep: &axiolid_brep::ExactBRep) -> f64 {
    let mut points: Vec<(f64, f64)> = brep
        .topology()
        .vertices()
        .iter()
        .filter(|v| v.position.z.abs() < 1e-9)
        .map(|v| (v.position.x, v.position.y))
        .collect();
    points.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
    (0..points.len())
        .map(|i| {
            let (x0, y0) = points[i];
            let (x1, y1) = points[(i + 1) % points.len()];
            x0 * y1 - x1 * y0
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// The case #66 names: a general (non-half-space) exact boolean now succeeds.
///
/// Two overlapping 4x4 columns, offset by 2 in x. Their intersection is a
/// 2x4 region -- previously a typed refusal, now an exact solid.
#[test]
fn a_general_exact_intersection_now_succeeds() {
    let subject = prism(vec![rect_ring(0.0, 0.0, 4.0, 4.0)], 0.0, 3.0);
    let tool = prism(vec![rect_ring(2.0, 0.0, 4.0, 4.0)], 0.0, 3.0);

    let result = boolean_prisms_exact(
        &subject,
        &tool,
        BooleanOperator::Intersection,
        Tolerance::METRE,
    )
    .expect("coaxial prism intersection is exactly constructible");

    // Overlap spans x in [0, 2] and y in [-2, 2]: area 8, not the naive 16.
    assert!(
        (base_area(&result) - 8.0).abs() < 1e-9,
        "expected the 2x4 overlap area 8, got {}",
        base_area(&result)
    );
    assert!(
        result
            .surfaces()
            .iter()
            .all(|s| matches!(s, axiolid_surface::Surface::Plane(_))),
        "a prism boolean must stay planar-faced"
    );
}

/// A wall with an interior opening: the dominant building-model pattern.
///
/// The opening is narrower than the wall in BOTH plan directions, so it
/// leaves a hole rather than cutting the wall in two. The tool spans the full
/// height, so the result is a prism with a hole -- exactly representable.
#[test]
fn a_wall_with_an_interior_opening_differences_exactly() {
    let wall = prism(vec![rect_ring(0.0, 0.0, 10.0, 4.0)], 0.0, 3.0);
    let opening = prism(vec![rect_ring(0.0, 0.0, 2.0, 2.0)], 0.0, 3.0);

    let result = boolean_prisms_exact(
        &wall,
        &opening,
        BooleanOperator::Difference,
        Tolerance::METRE,
    )
    .expect("a full-height interior opening leaves one prism with a hole");

    // The base ring is the wall outline; the hole is a second ring. Area is
    // checked on the outer boundary, which the opening does not touch.
    let (width, height) = base_extent(&result);
    assert!(
        (width - 10.0).abs() < 1e-9 && (height - 4.0).abs() < 1e-9,
        "the outer boundary must be unchanged at 10 x 4, got {width} x {height}"
    );

    // A hole means more than one loop bounds the cap face.
    let cap_bounds = result
        .topology()
        .faces()
        .iter()
        .map(|f| f.bounds.len())
        .max()
        .expect("the solid has faces");
    assert!(
        cap_bounds >= 2,
        "the opening must appear as a hole loop on the cap, got {cap_bounds}"
    );
}

/// A union of prisms with differing spans is refused, not flattened.
///
/// The true result is stepped -- two cross-sections at two heights -- and a
/// single prism cannot represent it. Returning the taller or shorter span
/// would silently change the geometry.
#[test]
fn a_union_with_differing_spans_is_refused() {
    let short = prism(vec![rect_ring(0.0, 0.0, 4.0, 4.0)], 0.0, 1.0);
    let tall = prism(vec![rect_ring(2.0, 0.0, 4.0, 4.0)], 0.0, 5.0);

    let error = boolean_prisms_exact(&short, &tall, BooleanOperator::Union, Tolerance::METRE)
        .expect_err("a stepped union is not a prism");
    assert!(
        matches!(
            error,
            GeomError::UnsupportedInput {
                input: "exact prism union with differing extrusion spans",
                ..
            }
        ),
        "got {error:?}"
    );
}

/// A tool shorter than the subject leaves a step, so difference is refused.
#[test]
fn a_difference_with_a_short_tool_is_refused() {
    let subject = prism(vec![rect_ring(0.0, 0.0, 10.0, 4.0)], 0.0, 3.0);
    // Stops at z = 1.5, halfway up the subject.
    let tool = prism(vec![rect_ring(0.0, 0.0, 2.0, 2.0)], 0.0, 1.5);

    let error = boolean_prisms_exact(
        &subject,
        &tool,
        BooleanOperator::Difference,
        Tolerance::METRE,
    )
    .expect_err("a partial-height cut leaves a stepped solid");
    assert!(
        matches!(
            error,
            GeomError::UnsupportedInput {
                input: "exact prism difference with a tool shorter than the subject",
                ..
            }
        ),
        "got {error:?}"
    );
}

/// Differential: the exact result agrees with the mesh boolmesh oracle.
///
/// The two paths share no code -- boolmesh works on triangles, this works on
/// the planar overlay -- so agreement is real evidence rather than a
/// tautology. Volume is the comparable quantity: the exact solid is a prism,
/// so its volume is cross-section area times height.
#[test]
fn the_exact_result_agrees_with_the_mesh_oracle() {
    use axiolid_construct::extrude::extrude_profile;
    use axiolid_construct::profile::Rings;
    use axiolid_contracts::ExecutionOptions;
    use axiolid_core::Vec3;
    use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
    use axiolid_mesh_boolean_contract::MeshBoolean;

    let subject_ring = rect_ring(0.0, 0.0, 4.0, 4.0);
    let tool_ring = rect_ring(2.0, 0.0, 4.0, 4.0);
    let height = 3.0;

    // Exact path.
    let exact = boolean_prisms_exact(
        &prism(vec![subject_ring.clone()], 0.0, height),
        &prism(vec![tool_ring.clone()], 0.0, height),
        BooleanOperator::Intersection,
        Tolerance::METRE,
    )
    .expect("exact intersection");
    let exact_volume = base_area(&exact) * height;

    // Mesh path: build both prisms as meshes and intersect with boolmesh.
    let to_mesh = |ring: &[Point2]| {
        extrude_profile(
            &Rings {
                outer: ring.to_vec(),
                holes: Vec::new(),
            },
            Vec3::Z,
            height,
            Tolerance::METRE,
        )
        .expect("a rectangle extrudes")
    };
    let mesh_result = BoolmeshBoolean::new()
        .boolean(
            &to_mesh(&subject_ring),
            &to_mesh(&tool_ring),
            BooleanOperator::Intersection,
            &ExecutionOptions::new(Tolerance::METRE),
        )
        .expect("the mesh oracle intersects")
        .mesh;

    // Divergence-theorem volume: triangulation-invariant.
    let mesh_volume: f64 = mesh_result
        .indices
        .chunks_exact(3)
        .map(|t| {
            let a = mesh_result.positions[t[0] as usize];
            let b = mesh_result.positions[t[1] as usize];
            let c = mesh_result.positions[t[2] as usize];
            a.dot(b.cross(c))
        })
        .sum::<f64>()
        / 6.0;

    assert!(
        (exact_volume - mesh_volume.abs()).abs() < 1e-6,
        "exact and mesh paths disagree: {exact_volume} vs {}",
        mesh_volume.abs()
    );
    // And both must equal the hand-computed 2x4x3.
    assert!(
        (exact_volume - 24.0).abs() < 1e-9,
        "expected 24, got {exact_volume}"
    );
}
