//! Independent analytic fixtures for the scalar mesh plane-section oracle.

use axiolid_core::{Frame3, Point2, Point3, Tolerance, Vec3};
use axiolid_kernel::{
    CancellationToken, ExecutionOptions, GeomError, MeshPlaneSectionRegistry, SectionLimits,
};
use axiolid_mesh::TriMesh;
use axiolid_scalar::ScalarSection;

fn cube() -> TriMesh {
    TriMesh::new(
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ],
        vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ],
    )
}

fn limits() -> SectionLimits {
    SectionLimits::new(100, 100, 100, 10)
}

fn horizontal(z: f64) -> Frame3 {
    Frame3 {
        origin: Point3::new(0.0, 0.0, z),
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
    }
}

fn registry() -> MeshPlaneSectionRegistry {
    let mut registry = MeshPlaneSectionRegistry::new();
    registry.register(0, ScalarSection::new());
    registry
}

#[test]
fn transverse_cube_section_is_the_analytic_unit_square() {
    let result = registry()
        .section(
            &cube(),
            horizontal(0.5),
            limits(),
            &ExecutionOptions::new(Tolerance::METRE),
        )
        .expect("transverse section");

    assert_eq!(result.contours.len(), 1);
    let contour = &result.contours[0];
    assert!(contour.is_closed());
    assert_eq!(
        contour.points.len(),
        4,
        "triangulation diagonals must not leak redundant collinear plan vertices"
    );
    for expected in [
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 0.0),
        Point2::new(1.0, 1.0),
        Point2::new(0.0, 1.0),
    ] {
        assert!(
            contour.points.contains(&expected),
            "missing analytic corner {expected:?}: {:?}",
            contour.points
        );
    }
    assert!(signed_area(&contour.points) > 0.0, "canonical CCW output");
    assert!(result.evidence.is_derived_from_input_mesh());
    assert_eq!(result.evidence.source_triangles, 12);
    assert_eq!(result.evidence.output_vertices, 4);
}

#[test]
fn exact_on_plane_mesh_edges_still_form_one_closed_loop() {
    let inverse_sqrt_two = 1.0 / 2.0_f64.sqrt();
    let frame = Frame3 {
        origin: Point3::ZERO,
        x: Vec3::new(inverse_sqrt_two, 0.0, inverse_sqrt_two),
        y: -Vec3::Y,
        z: Vec3::new(inverse_sqrt_two, 0.0, -inverse_sqrt_two),
    };
    let result = registry()
        .section(
            &cube(),
            frame,
            limits(),
            &ExecutionOptions::new(Tolerance::METRE),
        )
        .expect("diagonal section through mesh edges");

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].points.len(), 4);
    assert!(signed_area(&result.contours[0].points) > 0.0);
}

#[test]
fn tangent_vertex_and_missed_solid_return_empty_sections() {
    let inverse_sqrt_three = 1.0 / 3.0_f64.sqrt();
    let normal = Vec3::splat(inverse_sqrt_three);
    let x = Vec3::new(inverse_sqrt_three, -inverse_sqrt_three, 0.0).normalize();
    let y = normal.cross(x);
    let tangent = Frame3 {
        origin: Point3::ZERO,
        x,
        y,
        z: normal,
    };
    let options = ExecutionOptions::new(Tolerance::METRE);

    assert!(registry()
        .section(&cube(), tangent, limits(), &options)
        .expect("point tangent")
        .contours
        .is_empty());
    assert!(registry()
        .section(&cube(), horizontal(2.0), limits(), &options)
        .expect("miss")
        .contours
        .is_empty());
}

#[test]
fn coplanar_faces_are_refused_instead_of_becoming_arbitrary_curves() {
    let error = registry()
        .section(
            &cube(),
            horizontal(0.0),
            limits(),
            &ExecutionOptions::new(Tolerance::METRE),
        )
        .unwrap_err();
    assert!(matches!(error, GeomError::Degenerate(_)), "{error:?}");
}

#[test]
fn output_limits_and_cancellation_fail_before_partial_results_escape() {
    let error = registry()
        .section(
            &cube(),
            horizontal(0.5),
            SectionLimits::new(100, 100, 3, 10),
            &ExecutionOptions::new(Tolerance::METRE),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        GeomError::BudgetExceeded {
            resource: "section output vertices"
        }
    ));

    let token = CancellationToken::new();
    token.cancel();
    let options = ExecutionOptions::new(Tolerance::METRE).with_cancellation(token);
    assert_eq!(
        registry().section(&cube(), horizontal(0.5), limits(), &options),
        Err(GeomError::Cancelled)
    );
}

fn signed_area(points: &[Point2]) -> f64 {
    let mut twice = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        twice += current.x * next.y - current.y * next.x;
    }
    twice * 0.5
}
