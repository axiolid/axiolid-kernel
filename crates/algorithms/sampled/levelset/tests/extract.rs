//! Level-set extraction: closed, manifold, deterministic, convergent.

use axiolid_core::{Aabb, Point3, Scalar, Tolerance};
use axiolid_levelset::{level_set, LevelSetError};
use axiolid_measure::volume_properties;
use axiolid_mesh::audit_mesh;

fn bounds(half: Scalar) -> Aabb {
    let mut box3 = Aabb::default();
    box3.extend(Point3::new(-half, -half, -half));
    box3.extend(Point3::new(half, half, half));
    box3
}

fn sphere_sdf(radius: Scalar) -> impl Fn(Point3) -> Scalar {
    move |p: Point3| (p.x * p.x + p.y * p.y + p.z * p.z).sqrt() - radius
}

/// The contract worth having: the output is closed and two-manifold.
#[test]
fn an_extracted_mesh_is_closed_and_two_manifold() {
    let mesh = level_set(sphere_sdf(1.0), bounds(1.5), 0.25, 0.0).expect("extracts");
    let health = audit_mesh(&mesh, Tolerance::METRE);
    assert!(
        health.is_closed_two_manifold(),
        "extraction must be closed and manifold: boundary={} non_manifold={} winding={}",
        health.boundary_edges,
        health.non_manifold_edges,
        health.inconsistent_winding_edges
    );
}

/// A surface running to the edge of the bounds still closes.
///
/// This is what the padding shell buys. Without it the mesh is clipped into
/// an open sheet exactly when the caller's bounds are tight.
#[test]
fn a_surface_reaching_the_bounds_still_closes() {
    // Bounds barely larger than the sphere, so the surface reaches the
    // padding shell but no grid plane is exactly tangent to it.
    let mesh = level_set(sphere_sdf(1.0), bounds(1.05), 0.2, 0.0).expect("extracts");
    let health = audit_mesh(&mesh, Tolerance::METRE);
    assert!(
        health.is_closed_two_manifold(),
        "a surface at the bounds must still close, boundary edges: {}",
        health.boundary_edges
    );
}

/// Volume converges on the analytic sphere as the grid refines.
#[test]
fn volume_converges_as_the_edge_length_halves() {
    let radius = 1.0;
    let exact = 4.0 / 3.0 * std::f64::consts::PI * radius * radius * radius;

    let mut previous = Scalar::INFINITY;
    for edge in [0.4, 0.2, 0.1] {
        let mesh = level_set(sphere_sdf(radius), bounds(1.45), edge, 0.0).expect("extracts");
        let measured = volume_properties(&mesh, Tolerance::METRE)
            .expect("measures")
            .signed_volume
            .abs();
        let error = (measured - exact).abs() / exact;
        assert!(
            error < previous,
            "error must strictly decrease as the grid refines: {error} !< {previous} at edge {edge}"
        );
        previous = error;
    }
    assert!(
        previous < 0.02,
        "the finest grid should be within 2% of the analytic sphere, got {previous}"
    );
}

/// Identical inputs give byte-identical output.
#[test]
fn extraction_is_deterministic() {
    let first = level_set(sphere_sdf(1.0), bounds(1.45), 0.3, 0.0).expect("extracts");
    let second = level_set(sphere_sdf(1.0), bounds(1.45), 0.3, 0.0).expect("extracts");
    assert_eq!(first.indices, second.indices, "index buffer must match");
    assert_eq!(
        first.positions, second.positions,
        "vertex positions must be bit-identical"
    );
}

/// A field that never crosses the level refuses; it does not return nothing.
#[test]
fn an_empty_field_is_refused_rather_than_returning_no_triangles() {
    // Everywhere positive: no surface exists at level 0.
    let error = level_set(|_| 5.0, bounds(1.0), 0.25, 0.0).expect_err("must refuse");
    assert!(
        matches!(error, LevelSetError::NoCrossing { .. }),
        "an absent surface must be named, got {error:?}"
    );
}

/// The level is honoured, not assumed to be zero.
#[test]
fn a_non_zero_level_extracts_the_right_offset_surface() {
    // The 0.5 level set of a unit sphere SDF is a sphere of radius 1.5.
    let mesh = level_set(sphere_sdf(1.0), bounds(2.05), 0.15, 0.5).expect("extracts");
    // Measured by extent rather than volume: `volume_properties` requires a
    // closed mesh, and closedness is asserted separately by
    // `an_extracted_mesh_is_closed_and_two_manifold`. Testing the radius
    // here keeps this test about the LEVEL being honoured.
    let radius = mesh
        .positions
        .iter()
        .map(|p| (p.x * p.x + p.y * p.y + p.z * p.z).sqrt())
        .fold(0.0_f64, f64::max);
    assert!(
        (radius - 1.5).abs() < 0.05,
        "the 0.5 level of a unit-sphere SDF must be a radius-1.5 sphere, got {radius}"
    );
}

#[test]
fn invalid_requests_are_refused_by_name() {
    assert!(matches!(
        level_set(sphere_sdf(1.0), bounds(1.0), 0.0, 0.0),
        Err(LevelSetError::InvalidEdgeLength(_))
    ));
    assert!(matches!(
        level_set(sphere_sdf(1.0), bounds(1.0), -1.0, 0.0),
        Err(LevelSetError::InvalidEdgeLength(_))
    ));
    assert!(matches!(
        level_set(sphere_sdf(1.0), bounds(1.0), 0.25, Scalar::NAN),
        Err(LevelSetError::InvalidLevel(_))
    ));
    assert!(matches!(
        level_set(sphere_sdf(1.0), Aabb::default(), 0.25, 0.0),
        Err(LevelSetError::InvalidBounds)
    ));
}

/// A field that misbehaves is reported, not silently meshed.
#[test]
fn a_non_finite_sample_is_refused() {
    let error = level_set(
        |p: Point3| if p.x > 0.0 { Scalar::NAN } else { -1.0 },
        bounds(1.0),
        0.25,
        0.0,
    )
    .expect_err("must refuse");
    assert!(
        matches!(error, LevelSetError::NonFiniteSample { .. }),
        "a non-finite sample must be named, got {error:?}"
    );
}

/// An impossible request is refused before it exhausts memory.
#[test]
fn an_over_budget_grid_is_refused_up_front() {
    let error = level_set(sphere_sdf(1.0), bounds(1000.0), 0.001, 0.0).expect_err("must refuse");
    assert!(
        matches!(error, LevelSetError::BudgetExceeded { .. }),
        "an oversized grid must be refused, got {error:?}"
    );
}

/// The sign convention must be honoured, not merely consistent.
///
/// Inverting `inside` still yields a closed manifold mesh of the same
/// triangles, so every structural test passes either way. What changes is
/// the ORIENTATION: a mesh enclosing the sphere has positive signed volume
/// under the right-hand rule, while the inverted one reports negative.
/// Without this, the sign convention is untested.
#[test]
fn the_enclosed_side_is_the_one_below_the_level() {
    let mesh = level_set(sphere_sdf(1.0), bounds(1.45), 0.25, 0.0).expect("extracts");
    let signed = volume_properties(&mesh, Tolerance::METRE)
        .expect("measures")
        .signed_volume;
    assert!(
        signed > 0.0,
        "the mesh must enclose the region BELOW the level, signed volume {signed}"
    );
    let exact = 4.0 / 3.0 * std::f64::consts::PI;
    assert!(
        (signed - exact).abs() / exact < 0.1,
        "and it must be the sphere's own volume, got {signed} against {exact}"
    );
}
