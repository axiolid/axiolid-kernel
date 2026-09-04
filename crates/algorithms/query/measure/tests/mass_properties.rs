//! Mass properties against closed-form oracles (#72).
//!
//! Every expectation here is an independently derived analytic value, not a
//! second implementation of the same sum. A box's second moment about the
//! origin is `V * (L^2 / 3)` per axis for a box cornered at the origin, and
//! the parallel-axis theorem gives the offset case.

use axiolid_core::{Point3, Tolerance, Vec3};
use axiolid_measure::{MassProperties, Measure, MeshMeasure};
use axiolid_mesh::TriMesh;

fn tol() -> Tolerance {
    Tolerance::new(1e-6, 1e-9).expect("valid tolerance")
}

/// Axis-aligned box from `min` to `max`, outward-oriented.
///
/// Winding matches the conformance suite's `box_at`, whose orientation is
/// already pinned by the boolean provider tests.
fn box_mesh(min: [f64; 3], max: [f64; 3]) -> TriMesh {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let positions = vec![
        [x0, y0, z0].into(),
        [x1, y0, z0].into(),
        [x1, y1, z0].into(),
        [x0, y1, z0].into(),
        [x0, y0, z1].into(),
        [x1, y0, z1].into(),
        [x1, y1, z1].into(),
        [x0, y1, z1].into(),
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6,
        3, 0, 4, 3, 4, 7,
    ];
    TriMesh::new(positions, indices)
}

fn measure(mesh: &TriMesh) -> MassProperties {
    MeshMeasure
        .measure(mesh, tol())
        .expect("a closed box is measurable")
}

/// Volume, area and centroid match the closed forms for a unit cube.
#[test]
fn a_unit_cube_matches_its_closed_forms() {
    let props = measure(&box_mesh([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));

    assert!(
        (props.signed_volume - 1.0).abs() < 1e-12,
        "unit cube volume: {}",
        props.signed_volume
    );
    assert!(
        (props.area - 6.0).abs() < 1e-12,
        "unit cube area is 6 faces of 1: {}",
        props.area
    );
    let centre = Point3::new(0.5, 0.5, 0.5);
    assert!(
        (props.centroid - centre).length() < 1e-12,
        "unit cube centroid: {:?}",
        props.centroid
    );
}

/// A non-cubic box: catches an implementation that only works when the
/// three extents happen to be equal.
#[test]
fn an_oblong_box_matches_its_closed_forms() {
    let (lx, ly, lz) = (3.0, 5.0, 7.0);
    let props = measure(&box_mesh([0.0, 0.0, 0.0], [lx, ly, lz]));

    let expected_volume = lx * ly * lz;
    assert!(
        (props.signed_volume - expected_volume).abs() < 1e-9,
        "expected {expected_volume}, got {}",
        props.signed_volume
    );
    let expected_area = 2.0 * (lx * ly + ly * lz + lx * lz);
    assert!(
        (props.area - expected_area).abs() < 1e-9,
        "expected {expected_area}, got {}",
        props.area
    );
}

/// Second moments match the closed form, per axis.
///
/// For a box cornered at the origin with extents `L`, the integral of `x^2`
/// over the solid is `V * Lx^2 / 3`. Distinct extents mean a swapped or
/// shared axis cannot pass.
#[test]
fn second_moments_match_the_closed_form() {
    let (lx, ly, lz) = (3.0, 5.0, 7.0);
    let props = measure(&box_mesh([0.0, 0.0, 0.0], [lx, ly, lz]));
    let volume = lx * ly * lz;

    let expected = Vec3::new(
        volume * lx * lx / 3.0,
        volume * ly * ly / 3.0,
        volume * lz * lz / 3.0,
    );
    for (axis, name) in [(0, "x"), (1, "y"), (2, "z")] {
        let got = props.second_moment_diagonal[axis];
        let want = expected[axis];
        assert!(
            (got - want).abs() < 1e-9 * want.abs().max(1.0),
            "{name} second moment: expected {want}, got {got}"
        );
    }
}

/// The parallel-axis theorem holds for a translated solid.
///
/// This is the strongest available check: it relates two independent
/// measurements through a law neither of them knows about.
#[test]
fn second_moments_obey_the_parallel_axis_theorem() {
    let (lx, ly, lz) = (2.0, 3.0, 4.0);
    let shift = Vec3::new(10.0, -7.0, 4.5);

    let at_origin = measure(&box_mesh([0.0, 0.0, 0.0], [lx, ly, lz]));
    let moved = measure(&box_mesh(
        [shift.x, shift.y, shift.z],
        [shift.x + lx, shift.y + ly, shift.z + lz],
    ));

    let volume = lx * ly * lz;
    for (axis, name) in [(0, "x"), (1, "y"), (2, "z")] {
        // Second moment about the origin of the shifted body equals the
        // original plus V * (2 * c_old * d + d^2), from expanding (t + d)^2.
        let c_old = at_origin.centroid[axis];
        let d = shift[axis];
        let want = at_origin.second_moment_diagonal[axis] + volume * (2.0 * c_old * d + d * d);
        let got = moved.second_moment_diagonal[axis];
        assert!(
            (got - want).abs() < 1e-8 * want.abs().max(1.0),
            "{name}: parallel-axis expected {want}, got {got}"
        );
    }
}

/// An open shell is refused, not measured.
#[test]
fn an_open_shell_is_refused() {
    // A single triangle: real area, no enclosed volume.
    let mesh = TriMesh::new(
        vec![
            [0.0, 0.0, 0.0].into(),
            [1.0, 0.0, 0.0].into(),
            [0.0, 1.0, 0.0].into(),
        ],
        vec![0, 1, 2],
    );

    let error = MeshMeasure
        .measure(&mesh, tol())
        .expect_err("an open shell has no mass properties");
    let text = format!("{error:?}");
    assert!(
        text.contains("MeshNotVolumeUsable"),
        "refusal must name the volume-usability failure, got: {text}"
    );
}
