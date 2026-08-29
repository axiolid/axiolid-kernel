//! Mesh interference against hand-checkable arrangements (#4).
//!
//! Every fixture here is a pair of boxes whose relationship is obvious on
//! paper: clearly apart, sharing a face, or overlapping by a known amount. A
//! clash engine that cannot get these right is worthless on a real model, and
//! one that cannot tell contact from overlap floods a report with every
//! abutting wall.

use axiolid_core::{Point3, Tolerance};
use axiolid_mesh::TriMesh;
use axiolid_scalar::clash::{interference, Interference};

/// Axis-aligned box as a closed, outward-oriented triangle mesh.
fn box_mesh(min: Point3, max: Point3) -> TriMesh {
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let positions = vec![
        p(min.x, min.y, min.z),
        p(max.x, min.y, min.z),
        p(max.x, max.y, min.z),
        p(min.x, max.y, min.z),
        p(min.x, min.y, max.z),
        p(max.x, min.y, max.z),
        p(max.x, max.y, max.z),
        p(min.x, max.y, max.z),
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, // bottom
        4, 5, 6, 4, 6, 7, // top
        0, 1, 5, 0, 5, 4, // -y
        1, 2, 6, 1, 6, 5, // +x
        2, 3, 7, 2, 7, 6, // +y
        3, 0, 4, 3, 4, 7, // -x
    ];
    TriMesh::new(positions, indices)
}

fn unit_box_at(x: f64) -> TriMesh {
    box_mesh(Point3::new(x, 0.0, 0.0), Point3::new(x + 1.0, 1.0, 1.0))
}

// --- the three verdicts -----------------------------------------------------

#[test]
fn separated_solids_are_clear() {
    let a = unit_box_at(0.0);
    let b = unit_box_at(5.0);
    let report = interference(&a, &b, Tolerance::MILLIMETRE).expect("interference");
    assert_eq!(report.kind, Interference::Clear);
    assert!(report.is_clear());
    assert!(report.penetrating_pairs.is_empty());
    assert!(report.touching_pairs.is_empty());
}

#[test]
fn overlapping_solids_penetrate() {
    // b starts at 0.5, inside a's [0,1] span: the solids share volume.
    let a = unit_box_at(0.0);
    let b = unit_box_at(0.5);
    let report = interference(&a, &b, Tolerance::MILLIMETRE).expect("interference");
    assert_eq!(report.kind, Interference::Penetrating);
    assert!(report.is_penetrating());
    // The evidence for THIS arrangement is containment, not crossing pairs:
    // two boxes overlapping face-to-face meet only edge-to-edge, which the
    // exact predicate reports as contact. The verdict must still be
    // penetration, and the report must say which test produced it.
    assert!(
        report.containment,
        "a face-to-face overlap is decided by containment"
    );
}

#[test]
fn face_to_face_solids_touch_but_do_not_penetrate() {
    // This is the distinction that decides whether a model checker is usable:
    // every abutting wall, slab and column pair in a building lands here. If
    // contact reads as a clash the report is noise.
    let a = unit_box_at(0.0);
    let b = unit_box_at(1.0);
    let report = interference(&a, &b, Tolerance::MILLIMETRE).expect("interference");
    assert_eq!(
        report.kind,
        Interference::Touching,
        "shared face must be contact, not penetration"
    );
    assert!(report.penetrating_pairs.is_empty());
    assert!(!report.touching_pairs.is_empty());
}

// --- the exactness claim ----------------------------------------------------

#[test]
fn a_hair_of_separation_is_still_clear() {
    // 1e-12 apart with a zero tolerance: an inexact predicate rounds this to
    // contact. The verdict is a topological fact about the coordinates given.
    let a = unit_box_at(0.0);
    let b = unit_box_at(1.0 + 1e-12);
    let report = interference(&a, &b, Tolerance::ZERO).expect("interference");
    assert_eq!(
        report.kind,
        Interference::Clear,
        "1e-12 gap must not read as contact under an exact predicate"
    );
}

#[test]
fn a_hair_of_overlap_is_still_penetration() {
    let a = unit_box_at(0.0);
    let b = unit_box_at(1.0 - 1e-9);
    let report = interference(&a, &b, Tolerance::ZERO).expect("interference");
    assert_eq!(
        report.kind,
        Interference::Penetrating,
        "1e-9 overlap must not be rounded away"
    );
}

// --- broad phase actually does work -----------------------------------------

#[test]
fn the_broad_phase_rejects_the_quadratic_majority() {
    // 12 x 12 = 144 candidate pairs. If the broad phase is not culling, this
    // number is the exact-predicate call count and the design has no value.
    let a = unit_box_at(0.0);
    let b = unit_box_at(5.0);
    let report = interference(&a, &b, Tolerance::MILLIMETRE).expect("interference");
    assert_eq!(
        report.narrow_phase_tests + report.broad_phase_rejections,
        144,
        "every pair must be either tested or rejected"
    );
    assert_eq!(
        report.narrow_phase_tests, 0,
        "well-separated solids need no exact test at all"
    );
}

#[test]
fn tolerance_widens_the_broad_phase_without_changing_the_verdict() {
    // Offset in y as well as x so the two boxes share no plane at all: boxes
    // that merely slide along x still have coplanar side faces (both sit on
    // y=0 and z=0), and coplanar overlapping faces ARE contact. That is
    // correct geometry, so the fixture must avoid it to isolate the property
    // under test.
    let a = box_mesh(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
    let b = box_mesh(Point3::new(1.05, 2.0, 2.0), Point3::new(2.05, 3.0, 3.0));

    let tight = interference(&a, &b, Tolerance::ZERO).expect("interference");
    let loose =
        interference(&a, &b, Tolerance::new(3.0, 1e-9).expect("tolerance")).expect("interference");

    assert!(
        loose.narrow_phase_tests > tight.narrow_phase_tests,
        "a wider tolerance must test more pairs: {} vs {}",
        loose.narrow_phase_tests,
        tight.narrow_phase_tests
    );
    assert_eq!(tight.kind, Interference::Clear);
    assert_eq!(
        loose.kind,
        Interference::Clear,
        "tolerance widens the search, it must not change the verdict"
    );
}

// --- honesty about untestable input -----------------------------------------

#[test]
fn a_degenerate_triangle_is_counted_not_silently_ignored() {
    // A sliver cannot be classified exactly. Reporting it as clear would be a
    // false negative in exactly the place a checker must not have one.
    let a = unit_box_at(0.0);
    let mut b = unit_box_at(0.5);
    // Collapse one triangle onto a line.
    let first = b.indices[0] as usize;
    b.indices[1] = first as u32;
    let report = interference(&a, &b, Tolerance::MILLIMETRE).expect("interference");
    assert!(
        report.degenerate_skips > 0,
        "a collapsed triangle must be reported as unclassifiable"
    );
}

#[test]
fn a_non_finite_tolerance_is_refused() {
    let a = unit_box_at(0.0);
    let b = unit_box_at(0.5);
    let bad = Tolerance::new(f64::NAN, 1e-9);
    // The constructor may already refuse it; if it does not, interference must.
    if let Ok(t) = bad {
        assert!(interference(&a, &b, t).is_err());
    }
}

#[test]
fn an_empty_mesh_clashes_with_nothing() {
    let a = unit_box_at(0.0);
    let empty = TriMesh::new(Vec::new(), Vec::new());
    let report = interference(&a, &empty, Tolerance::MILLIMETRE).expect("interference");
    assert_eq!(report.kind, Interference::Clear);
    assert_eq!(report.narrow_phase_tests, 0);
}

#[test]
fn a_solid_fully_inside_another_is_penetrating() {
    // Zero triangle pairs meet, yet this is the most severe interference
    // there is. Surface intersection alone reports it as Clear.
    let outer = box_mesh(Point3::new(0.0, 0.0, 0.0), Point3::new(10.0, 10.0, 10.0));
    let inner = box_mesh(Point3::new(4.0, 4.0, 4.0), Point3::new(6.0, 6.0, 6.0));
    let report = interference(&outer, &inner, Tolerance::MILLIMETRE).expect("interference");
    assert_eq!(
        report.kind,
        Interference::Penetrating,
        "a contained solid must not read as clear"
    );
    assert!(report.containment, "verdict came from containment");
    assert!(
        report.penetrating_pairs.is_empty(),
        "no surfaces cross in this arrangement"
    );
}
