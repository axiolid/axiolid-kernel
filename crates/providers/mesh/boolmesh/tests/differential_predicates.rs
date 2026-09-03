//! Differential check of boolmesh output against certified predicates (#24).
//!
//! The provider decides orientation and inside/outside with floating-point
//! arithmetic. This gate re-decides the same questions with the exact,
//! certified `orient3d` from `axiolid-predicates` and fails on disagreement,
//! so the provider's sign decisions are *validated* rather than trusted.
//!
//! # Why this is a real oracle
//!
//! `orient3d` shares no code with boolmesh: it is Shewchuk-style adaptive
//! expansion arithmetic that returns an exact sign for finite binary64 input,
//! or `Uncertain` rather than a guess. A disagreement is therefore evidence
//! about the provider, not about a shared rounding convention. The provider and
//! `axiolid-mesh` both compute in plain `f64`, so a self-check using the
//! provider's own arithmetic could not detect a sign error at all.
//!
//! # Scope, and why it is narrow on purpose
//!
//! The invariant "every face is wound away from one interior point" holds only
//! for CONVEX solids. An earlier revision of this file asserted it for every
//! boolean result and failed on the non-convex ones — correctly, because a
//! notched box genuinely has faces on both sides of its centroid. Keeping that
//! assertion would have meant either weakening it until it proved nothing, or
//! recording a provider bug that does not exist.
//!
//! So the exact checks are applied where they are actually valid:
//!
//! - convexity-preserving operations (intersection of convex operands) are
//!   certified convex, vertex by vertex, against every face plane;
//! - inside/outside classification is differentially checked for those convex
//!   results against independently computed ground truth;
//! - winding is audited for convex results, where the one-interior-point test
//!   is sound.
//!
//! Non-convex results are covered by the volume-conservation and structural
//! gates next door; this file does not pretend to certify them.

mod support;

use axiolid_contracts::{ExecutionOptions, Sign};
use axiolid_core::{BooleanOperator, Point3, Tolerance};
use axiolid_guarantees::Certified;
use axiolid_mesh::TriMesh;
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_boolean_contract::MeshBoolean;
use axiolid_predicates::orient3d;
use support::{boxx, inverted, volume};

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::METRE)
}

/// Centroid of the mesh vertices. Strictly interior for a convex solid.
fn centroid(mesh: &TriMesh) -> Point3 {
    let mut sum = Point3::new(0.0, 0.0, 0.0);
    for position in &mesh.positions {
        sum = Point3::new(sum.x + position.x, sum.y + position.y, sum.z + position.z);
    }
    let count = mesh.positions.len() as f64;
    Point3::new(sum.x / count, sum.y / count, sum.z / count)
}

/// Exact sign of `probe` against the plane of triangle `index`, or `None` when
/// even the exact predicate cannot decide.
fn face_sign(mesh: &TriMesh, index: usize, probe: Point3) -> Option<Sign> {
    let corner = &mesh.indices[index * 3..index * 3 + 3];
    let a = mesh.positions[corner[0] as usize];
    let b = mesh.positions[corner[1] as usize];
    let c = mesh.positions[corner[2] as usize];
    match orient3d(a, b, c, probe) {
        Certified::Certain { sign, .. } => Some(sign),
        _ => None,
    }
}

/// Certify that every vertex lies on the interior side of every face plane.
///
/// This is the exact definition of convexity for a closed polyhedron, decided
/// by `orient3d` rather than by a tolerance. Vertices lying exactly ON a face
/// plane are expected and allowed — they are the face's own corners and the
/// coplanar neighbours produced by splitting a flat face into triangles.
///
/// Returns the number of faces actually decided, so a caller can reject a
/// vacuous pass.
fn certify_convex(mesh: &TriMesh, label: &str) -> usize {
    let interior = centroid(mesh);
    let face_count = mesh.indices.len() / 3;
    assert!(face_count > 0, "{label}: no faces to certify");

    let mut decided = 0;
    for face in 0..face_count {
        let Some(interior_sign) = face_sign(mesh, face, interior) else {
            panic!("{label}: exact orient3d could not place the centroid relative to face {face}");
        };
        assert_ne!(
            interior_sign,
            Sign::Zero,
            "{label}: the centroid is coplanar with face {face}, so this fixture is degenerate"
        );
        decided += 1;

        for (vertex, position) in mesh.positions.iter().enumerate() {
            let Some(sign) = face_sign(mesh, face, *position) else {
                panic!(
                    "{label}: exact orient3d could not place vertex {vertex} against face {face}"
                );
            };
            // On the plane is fine; on the far side from the interior is not.
            assert!(
                sign == interior_sign || sign == Sign::Zero,
                "{label}: vertex {vertex} lies outside the plane of face {face}, so the \
                 result is not convex — the provider produced a non-convex mesh from \
                 convex operands"
            );
        }
    }
    decided
}

/// Exact inside/outside test for a convex mesh, decided entirely by `orient3d`.
///
/// `None` when the point lies exactly on the boundary, which is neither inside
/// nor outside and must not be silently bucketed as either.
fn convex_contains(mesh: &TriMesh, probe: Point3) -> Option<bool> {
    let interior = centroid(mesh);
    let face_count = mesh.indices.len() / 3;
    let mut on_boundary = false;
    for face in 0..face_count {
        let interior_sign = face_sign(mesh, face, interior)?;
        let probe_sign = face_sign(mesh, face, probe)?;
        if probe_sign == Sign::Zero {
            on_boundary = true;
            continue;
        }
        if probe_sign != interior_sign {
            return Some(false);
        }
    }
    if on_boundary {
        None
    } else {
        Some(true)
    }
}

/// The intersection of two boxes is a box, so the result must certify convex.
///
/// Convexity is not incidental here: it is a property the operation must
/// preserve, and a provider that emitted a stray inverted or displaced face
/// would break it while still passing structural validation.
#[test]
fn intersection_of_convex_operands_is_certified_convex() {
    let subject = boxx(0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 0.0);
    let tool = boxx(0.8, 0.4, 0.5, 1.4, 1.4, 1.4, 0.0);

    let result = BoolmeshBoolean::new()
        .boolean(&subject, &tool, BooleanOperator::Intersection, &options())
        .expect("intersection of two boxes")
        .mesh;

    assert!(volume(&result) > 0.0, "the operands do overlap");
    let decided = certify_convex(&result, "box ∩ box");
    assert!(decided > 0, "the certification decided nothing");
}

/// Inside/outside decisions must match independently computed ground truth.
///
/// The true intersection of two axis-aligned boxes is the interval-wise
/// overlap, which is known in closed form. Sample points are classified by that
/// closed form and, separately, by exact `orient3d` sidedness against the
/// provider's mesh. Disagreement means the provider's inside/outside decision
/// is wrong, which is precisely what this issue asks to validate.
#[test]
fn inside_outside_agrees_with_closed_form_ground_truth() {
    // Axis-aligned, so the true intersection is the interval-wise overlap.
    // The bounds are DERIVED from the same numbers that build the boxes rather
    // than transcribed: an earlier revision hand-wrote them, got the x range
    // wrong, and blamed the provider for the discrepancy. Deriving them means
    // the ground truth cannot drift from the fixture.
    let (a_lo, a_hi) = (Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 2.0));
    let (b_lo, b_hi) = (
        Point3::new(0.8 - 0.7, 0.8 - 0.7, 0.5),
        Point3::new(0.8 + 0.7, 0.8 + 0.7, 0.5 + 1.0),
    );
    let subject = boxx(0.5, 0.5, 0.0, 1.0, 1.0, 2.0, 0.0);
    let tool = boxx(0.8, 0.8, 0.5, 1.4, 1.4, 1.0, 0.0);
    let lo = Point3::new(a_lo.x.max(b_lo.x), a_lo.y.max(b_lo.y), a_lo.z.max(b_lo.z));
    let hi = Point3::new(a_hi.x.min(b_hi.x), a_hi.y.min(b_hi.y), a_hi.z.min(b_hi.z));

    let result = BoolmeshBoolean::new()
        .boolean(&subject, &tool, BooleanOperator::Intersection, &options())
        .expect("intersection of two axis-aligned boxes")
        .mesh;
    certify_convex(&result, "axis-aligned ∩");

    // Deliberately away from faces, edges and corners: boundary points are
    // genuinely undecidable here and testing them would assert a convention
    // rather than a fact. The `expected` column is asserted against the derived
    // bounds below, so a typo here fails loudly instead of weakening the test.
    let probes = [
        (Point3::new(0.5, 0.5, 1.0), true),
        (Point3::new(0.15, 0.15, 0.55), true),
        (Point3::new(0.95, 0.95, 1.45), true),
        (Point3::new(0.05, 0.5, 1.0), false),
        (Point3::new(0.5, 1.2, 1.0), false),
        (Point3::new(0.5, 0.5, 0.4), false),
        (Point3::new(0.5, 0.5, 1.6), false),
        (Point3::new(5.0, 5.0, 5.0), false),
    ];

    let mut checked = 0;
    for (probe, expected) in probes {
        let truth = probe.x > lo.x
            && probe.x < hi.x
            && probe.y > lo.y
            && probe.y < hi.y
            && probe.z > lo.z
            && probe.z < hi.z;
        assert_eq!(truth, expected, "the fixture's own ground truth is wrong");

        let Some(decided) = convex_contains(&result, probe) else {
            panic!("probe {probe:?} landed on the boundary; pick an interior or exterior point");
        };
        assert_eq!(
            decided, truth,
            "certified orient3d says inside={decided} for {probe:?}, closed form says {truth}"
        );
        checked += 1;
    }
    assert_eq!(checked, probes.len(), "not every probe was checked");
}

/// Every face of a convex result is wound away from the interior.
///
/// Sound only because the result is convex; see the module docs.
#[test]
fn convex_result_winding_agrees_with_certified_orient3d() {
    let subject = boxx(0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 0.0);
    let tool = boxx(0.9, 0.9, 0.9, 1.6, 1.6, 1.6, 0.0);

    let result = BoolmeshBoolean::new()
        .boolean(&subject, &tool, BooleanOperator::Intersection, &options())
        .expect("intersection")
        .mesh;

    let interior = centroid(&result);
    let face_count = result.indices.len() / 3;
    let mut positive = 0;
    let mut negative = 0;
    for face in 0..face_count {
        match face_sign(&result, face, interior) {
            Some(Sign::Positive) => positive += 1,
            Some(Sign::Negative) => negative += 1,
            other => panic!("face {face} was not decided exactly: {other:?}"),
        }
    }
    assert!(
        positive == 0 || negative == 0,
        "certified orient3d reports {positive} positive and {negative} negative faces about \
         the interior of a CONVEX result, so at least one face is inverted"
    );
    assert!(positive + negative > 0, "no faces were audited");
}

/// The convexity certification must reject a non-convex mesh.
///
/// Without this, a bug making `certify_convex` vacuous would leave every test
/// above green while checking nothing. A notched box is structurally valid,
/// manifold, and genuinely non-convex.
#[test]
fn the_convexity_certification_rejects_a_notched_box() {
    let subject = boxx(0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 0.0);
    let tool = boxx(0.9, 0.9, 1.0, 1.0, 1.0, 1.0, 0.0);
    let notched = BoolmeshBoolean::new()
        .boolean(&subject, &tool, BooleanOperator::Difference, &options())
        .expect("difference")
        .mesh;

    assert!(
        notched.validate_structure().is_ok(),
        "the notched mesh must be structurally valid, otherwise the structural check \
         would be doing this test's work"
    );

    let outcome = std::panic::catch_unwind(|| certify_convex(&notched, "notched"));
    assert!(
        outcome.is_err(),
        "a notched box is not convex, so the certification must reject it"
    );
}

/// The winding audit must notice a single inverted face.
///
/// One bad face is the realistic provider bug. Flipping the whole mesh could be
/// survived by a sign-convention check through symmetry; one face cannot.
#[test]
fn the_winding_audit_catches_a_single_inverted_face() {
    let mut solid = boxx(0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 0.0);
    let interior = centroid(&solid);
    let reference = face_sign(&solid, 0, interior).expect("decidable");

    solid.indices.swap(1, 2);
    let flipped = face_sign(&solid, 0, interior).expect("decidable");

    assert_ne!(
        reference, flipped,
        "reversing a triangle must flip its certified sign, otherwise the audit is \
         not sensitive to winding at all"
    );
}

/// Inverting every face must flip every certified sign.
#[test]
fn inverting_the_mesh_flips_every_certified_sign() {
    let solid = boxx(0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 0.0);
    let flipped = inverted(&solid);
    assert!(flipped.validate_structure().is_ok());

    let face_count = solid.indices.len() / 3;
    let upright_interior = centroid(&solid);
    let flipped_interior = centroid(&flipped);
    for face in 0..face_count {
        let upright = face_sign(&solid, face, upright_interior).expect("decidable");
        let reversed = face_sign(&flipped, face, flipped_interior).expect("decidable");
        assert_ne!(
            upright, reversed,
            "face {face} kept its sign after inversion"
        );
    }
}
