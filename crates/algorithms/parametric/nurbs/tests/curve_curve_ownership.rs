//! Ownership, dedup, and contact classification for planar curve pairs.
//!
//! Milestone v0.3 #5. Each fixture below is a case where the pre-#5 contract
//! either double-counted a root or collapsed a provable contact class into a
//! bare `Unresolved`.

use axiolid_core::Point2;
use axiolid_curve::{BSplineCurve, KnotSpec};
use axiolid_nurbs::{
    intersect_curve2_certified, CertifiedCurveIntersection2, CertifiedCurveIntersectionOptions,
    CurveIntersectionDegeneracy,
};

/// A single-span Bezier of the given control points.
fn bezier(points: Vec<Point2>) -> BSplineCurve<Point2> {
    let degree = u16::try_from(points.len() - 1).unwrap();
    BSplineCurve {
        degree,
        control_points: points,
        knots: vec![0.0, 1.0],
        multiplicities: vec![u32::from(degree) + 1; 2],
        weights: None,
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: Some(false),
    }
}

/// A degree-1 curve with an interior knot, so its domain splits into two
/// adjacent Bezier cells that share the parameter `0.5` exactly.
fn split_line(start: Point2, mid: Point2, end: Point2) -> BSplineCurve<Point2> {
    BSplineCurve {
        degree: 1,
        control_points: vec![start, mid, end],
        knots: vec![0.0, 0.5, 1.0],
        multiplicities: vec![2, 1, 2],
        weights: None,
        knot_spec: KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    }
}

fn options(max_nodes: u32, max_depth: u16) -> CertifiedCurveIntersectionOptions {
    CertifiedCurveIntersectionOptions::new(1e-9, max_nodes, max_depth).unwrap()
}

#[test]
fn a_root_on_a_shared_cell_boundary_is_counted_once() {
    // The crossing sits exactly at parameter 0.5 of the split curve, which is
    // the closed endpoint of BOTH of its Bezier cells. Without boundary
    // ownership the same root is isolated twice, once per cell.
    let split = split_line(
        Point2::new(-1.0, 0.0),
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
    );
    let crossing = bezier(vec![Point2::new(0.0, -1.0), Point2::new(0.0, 1.0)]);

    let outcome = intersect_curve2_certified(&split, &crossing, options(100_000, 64)).unwrap();
    let CertifiedCurveIntersection2::Degenerate {
        classification,
        contacts,
        ..
    } = outcome
    else {
        panic!("a knot-boundary crossing is not strict-interior isolable");
    };

    // The crossing is PROVEN, not merely unresolved: the residual is not
    // excluded and the tangents are certainly non-parallel.
    assert_eq!(
        classification,
        CurveIntersectionDegeneracy::BoundaryCrossing
    );
    assert_eq!(
        contacts.len(),
        1,
        "a shared-endpoint root was reported {} times",
        contacts.len()
    );

    // Ownership is the fused boundary box, which must contain the knot.
    let owned = contacts[0].parameters;
    assert!(owned.first.start <= 0.5 && 0.5 <= owned.first.end);
}

#[test]
fn two_distinct_roots_are_not_collapsed_by_dedup() {
    // Dedup must not become "report one root": these two crossings are far
    // apart in both parameter domains and must both survive.
    let split = split_line(
        Point2::new(-1.0, 0.0),
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
    );
    let zigzag = bezier(vec![Point2::new(-0.5, -1.0), Point2::new(-0.5, 1.0)]);
    let other = bezier(vec![Point2::new(0.5, -1.0), Point2::new(0.5, 1.0)]);

    let left = intersect_curve2_certified(&split, &zigzag, options(100_000, 64)).unwrap();
    let right = intersect_curve2_certified(&split, &other, options(100_000, 64)).unwrap();

    for outcome in [left, right] {
        let CertifiedCurveIntersection2::Complete { intersections, .. } = outcome else {
            panic!("an interior transverse crossing must be complete");
        };
        assert_eq!(intersections.len(), 1);
    }
}

#[test]
fn a_tangential_contact_is_classified_not_left_unresolved() {
    // A parabola resting on a horizontal line: they touch at one point and the
    // tangents there are parallel, so Krawczyk can never contract. Before #5
    // this fell out of the search as a bare `Unresolved` box.
    let line = bezier(vec![Point2::new(-1.0, 0.0), Point2::new(1.0, 0.0)]);
    let parabola = bezier(vec![
        Point2::new(-1.0, 1.0),
        Point2::new(0.0, -1.0),
        Point2::new(1.0, 1.0),
    ]);

    let outcome = intersect_curve2_certified(&line, &parabola, options(100_000, 12)).unwrap();
    let CertifiedCurveIntersection2::Degenerate {
        classification,
        contacts,
        ..
    } = outcome
    else {
        panic!("a tangential touch is not a transverse crossing");
    };

    assert_eq!(classification, CurveIntersectionDegeneracy::Tangency);
    assert!(contacts
        .iter()
        .any(|contact| contact.classification == CurveIntersectionDegeneracy::Tangency));
}

#[test]
fn a_partial_collinear_overlap_outranks_mere_touching() {
    // Two collinear segments sharing a positive-length span. The summary class
    // must be `Overlap`, which is strictly stronger than the endpoint contact
    // a naive box test would report.
    let first = bezier(vec![Point2::new(0.0, 0.0), Point2::new(2.0, 0.0)]);
    let second = bezier(vec![Point2::new(1.0, 0.0), Point2::new(3.0, 0.0)]);

    let outcome = intersect_curve2_certified(&first, &second, options(10_000, 32)).unwrap();
    let CertifiedCurveIntersection2::Degenerate {
        classification,
        contacts,
        ..
    } = outcome
    else {
        panic!("a positive-length shared span is not an isolated root");
    };

    assert_eq!(classification, CurveIntersectionDegeneracy::Overlap);
    assert!(!contacts.is_empty());
}

#[test]
fn collinear_segments_meeting_at_one_endpoint_are_not_called_overlap() {
    // Touching, not overlapping: the shared set is a single point, so claiming
    // `Overlap` would assert a positive-dimensional contact that is not there.
    let first = bezier(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)]);
    let second = bezier(vec![Point2::new(1.0, 0.0), Point2::new(2.0, 0.0)]);

    let outcome = intersect_curve2_certified(&first, &second, options(10_000, 32)).unwrap();
    let CertifiedCurveIntersection2::Degenerate { classification, .. } = outcome else {
        panic!("collinear endpoint contact is a degenerate relationship");
    };

    assert_ne!(classification, CurveIntersectionDegeneracy::Overlap);
}

#[test]
fn every_reported_contact_box_is_distinct() {
    // Dedup is a contract, not an optimisation: no two reported contacts may
    // name the same closed product box.
    let curve = split_line(
        Point2::new(-1.0, 0.0),
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
    );

    let outcome = intersect_curve2_certified(&curve, &curve, options(10_000, 32)).unwrap();
    let CertifiedCurveIntersection2::Degenerate { contacts, .. } = outcome else {
        panic!("a self-pair is structurally degenerate");
    };

    for (index, contact) in contacts.iter().enumerate() {
        for other in &contacts[index + 1..] {
            assert_ne!(
                contact.parameters, other.parameters,
                "the same product box was reported twice"
            );
        }
    }
}

#[test]
fn an_unresolved_box_stays_an_explicit_reported_outcome() {
    // #5 requires that an unresolved box remain a REPORTED outcome. A starved
    // depth budget must not silently downgrade to "no intersections".
    let line = bezier(vec![Point2::new(-1.0, 0.0), Point2::new(1.0, 0.0)]);
    let wave = bezier(vec![
        Point2::new(-1.0, 0.5),
        Point2::new(-0.3, -2.0),
        Point2::new(0.3, 2.0),
        Point2::new(1.0, -0.5),
    ]);

    let outcome = intersect_curve2_certified(&line, &wave, options(64, 1)).unwrap();
    let CertifiedCurveIntersection2::Degenerate { contacts, .. } = outcome else {
        panic!("a starved budget must not claim a complete classification");
    };

    assert!(!contacts.is_empty());
}
