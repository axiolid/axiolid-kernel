//! Scalar CPU triangle coverage, sampling determinism, and multi-layer fixtures.

mod fixtures;

use axiolid_core::{Tolerance, Vec3};
use axiolid_field_ops::{
    sample_triangles_cpu, CpuCoverageProvider, LayeredFieldError, SurfaceFacing, Triangle3,
};

#[test]
fn coverage_emits_surface_hits_and_never_occupancy() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 6.0), 1.0);
    let field = sample_triangles_cpu(&config, &fixtures::quad_at(frame, 2.0, 3.0)).unwrap();

    for y in 0..2 {
        for x in 0..2 {
            let cell = field.cell(x, y).unwrap();
            assert_eq!(cell.surfaces().len(), 1, "cell ({x},{y})");
            assert!((cell.surfaces()[0].w() - 2.0).abs() < 1e-9);
            assert!(
                cell.occupancy().is_empty(),
                "a triangle must never fabricate occupied volume"
            );
        }
    }
    assert_eq!(field.evidence().surface_hits, 4);
    assert_eq!(field.evidence().occupancy_spans, 0);
    assert_eq!(field.evidence().cells_sampled, 4);
}

#[test]
fn coverage_is_frame_neutral_for_a_non_z_up_frame() {
    let frame = fixtures::x_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 6.0), 1.0);
    // The quad is built in the frame's own space, so a world-Z assumption in the
    // sampler would find nothing here.
    let field = sample_triangles_cpu(&config, &fixtures::quad_at(frame, 2.5, 3.0)).unwrap();
    assert_eq!(field.evidence().surface_hits, 4);
    for cell in field.cells() {
        assert_eq!(cell.surfaces().len(), 1);
        assert!((cell.surfaces()[0].w() - 2.5).abs() < 1e-9);
    }
}

#[test]
fn one_cell_holds_many_stacked_layers() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 8.0), 1.0);
    let field = sample_triangles_cpu(&config, &fixtures::three_stacked_slabs(frame, 3.0)).unwrap();

    let cell = field.cell(0, 0).unwrap();
    let layers: Vec<f64> = cell.surfaces().iter().map(|hit| hit.w()).collect();
    assert_eq!(
        layers.len(),
        3,
        "single-floor behaviour would collapse these"
    );
    assert!((layers[0] - 1.0).abs() < 1e-9);
    assert!((layers[1] - 3.0).abs() < 1e-9);
    assert!((layers[2] - 5.0).abs() < 1e-9);
    assert_eq!(field.evidence().multi_layer_cells, 4);
    assert_eq!(field.evidence().empty_cells, 0);
}

#[test]
fn empty_cells_are_reported_not_invented() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(3.0, 3.0, 6.0), 1.0);
    // A small quad covering only the first cell's centre.
    let quad = [
        Triangle3::new(
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(0.9, 0.0, 2.0),
            Vec3::new(0.9, 0.9, 2.0),
        ),
        Triangle3::new(
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(0.9, 0.9, 2.0),
            Vec3::new(0.0, 0.9, 2.0),
        ),
    ];
    let field = sample_triangles_cpu(&config, &quad).unwrap();
    assert_eq!(field.cell(0, 0).unwrap().surfaces().len(), 1);
    assert!(field.cell(2, 2).unwrap().is_empty());
    assert_eq!(field.evidence().empty_cells, 8);
}

#[test]
fn repeated_runs_are_bit_identical() {
    let frame = fixtures::x_up_frame();
    let config = fixtures::config(frame, Vec3::new(4.0, 4.0, 8.0), 0.5);
    let triangles = fixtures::three_stacked_slabs(frame, 5.0);

    let first = sample_triangles_cpu(&config, &triangles).unwrap();
    let second = sample_triangles_cpu(&config, &triangles).unwrap();
    let third = CpuCoverageProvider::new()
        .sample(&config, &triangles)
        .unwrap();

    assert_eq!(first, second, "identical input must give identical fields");
    assert_eq!(first, third, "provider and free function must agree");
    assert_eq!(first.evidence(), third.evidence());
}

#[test]
fn input_order_does_not_change_stored_layer_order() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 8.0), 1.0);
    let mut forward = fixtures::three_stacked_slabs(frame, 3.0);
    let mut reversed = forward.clone();
    reversed.reverse();

    let a = sample_triangles_cpu(&config, &forward).unwrap();
    let b = sample_triangles_cpu(&config, &reversed).unwrap();
    let layers = |f: &axiolid_field_ops::LayeredField| -> Vec<f64> {
        f.cell(0, 0)
            .unwrap()
            .surfaces()
            .iter()
            .map(|h| h.w())
            .collect()
    };
    assert_eq!(layers(&a), layers(&b));

    forward.truncate(2);
    assert_eq!(
        sample_triangles_cpu(&config, &forward)
            .unwrap()
            .evidence()
            .surface_hits,
        4
    );
}

#[test]
fn parallel_and_degenerate_triangles_are_reported_not_sampled() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 6.0), 1.0);

    // A vertical triangle is parallel to the sampling direction: its plane
    // contains the ray, so there is no single crossing to report.
    let parallel = Triangle3::new(
        Vec3::new(0.5, 0.0, 0.0),
        Vec3::new(0.5, 0.0, 4.0),
        Vec3::new(0.5, 2.0, 4.0),
    );
    let field = sample_triangles_cpu(&config, &[parallel]).unwrap();
    assert_eq!(field.evidence().parallel_triangles_skipped, 1);
    assert_eq!(field.evidence().surface_hits, 0);

    let degenerate = Triangle3::new(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);
    let field = sample_triangles_cpu(&config, &[degenerate]).unwrap();
    assert_eq!(field.evidence().degenerate_triangles, 1);
    assert_eq!(field.evidence().surface_hits, 0);
}

#[test]
fn hits_outside_the_local_bounds_are_reported_not_stored() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 3.0), 1.0);
    // The slab sits above the configured local-z window.
    let field = sample_triangles_cpu(&config, &fixtures::quad_at(frame, 9.0, 3.0)).unwrap();
    assert_eq!(field.evidence().surface_hits, 0);
    assert_eq!(field.evidence().out_of_bounds_hits, 8);
}

#[test]
fn edge_and_vertex_contacts_are_flagged_as_boundary_evidence() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 6.0), 1.0);
    // Triangle edge passes exactly through the (0,0) cell centre at (0.5, 0.5).
    let touching = Triangle3::new(
        Vec3::new(0.0, 1.0, 2.0),
        Vec3::new(1.0, 0.0, 2.0),
        Vec3::new(1.0, 1.0, 2.0),
    );
    let field = sample_triangles_cpu(&config, &[touching]).unwrap();
    assert_eq!(field.cell(0, 0).unwrap().surfaces().len(), 1);
    assert!(
        field.evidence().boundary_contacts >= 1,
        "an on-edge hit must be reported, not silently treated as interior"
    );
}

#[test]
fn non_finite_geometry_is_rejected() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 6.0), 1.0);
    let broken = Triangle3::new(
        Vec3::new(f64::NAN, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
    );
    assert_eq!(
        sample_triangles_cpu(&config, &[broken]),
        Err(LayeredFieldError::NonFiniteGeometry)
    );
}

#[test]
fn sample_budget_is_enforced_by_the_caller_not_a_fixed_cap() {
    use axiolid_core::Vec3 as V;
    use axiolid_field_ops::{FieldBounds, FieldConfig, FieldResourceBudget};
    let frame = fixtures::z_up_frame();
    let config = FieldConfig::new(
        frame,
        FieldBounds::new(V::ZERO, V::new(2.0, 2.0, 8.0)).unwrap(),
        1.0,
        Tolerance::METRE,
        // Four cells, three slabs each: twelve layers requested, three allowed.
        FieldResourceBudget::new(16, 3),
    )
    .unwrap();
    assert_eq!(
        sample_triangles_cpu(&config, &fixtures::three_stacked_slabs(frame, 3.0)),
        Err(LayeredFieldError::SampleBudgetExceeded)
    );
}

#[test]
fn occupancy_requires_a_closed_shell_and_reports_winding() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 8.0), 1.0);

    // An open single slab cannot become volume: one crossing is unbalanced.
    let open = sample_triangles_cpu(&config, &fixtures::quad_at(frame, 2.0, 3.0)).unwrap();
    assert_eq!(
        open.derive_occupancy(Tolerance::METRE),
        Err(LayeredFieldError::UnbalancedCrossings)
    );

    // A closed slab yields exactly one span per cell, with correct winding.
    let closed =
        sample_triangles_cpu(&config, &fixtures::closed_slab(frame, 1.0, 3.0, 3.0)).unwrap();
    let cell = closed.cell(0, 0).unwrap();
    assert_eq!(cell.surfaces().len(), 2);
    assert_eq!(cell.surfaces()[0].facing(), SurfaceFacing::AgainstNormal);
    assert_eq!(cell.surfaces()[1].facing(), SurfaceFacing::WithNormal);

    let solid = closed.derive_occupancy(Tolerance::METRE).unwrap();
    let occupied = solid.cell(0, 0).unwrap().occupancy();
    assert_eq!(occupied.len(), 1);
    assert!((occupied[0].start - 1.0).abs() < 1e-9);
    assert!((occupied[0].end - 3.0).abs() < 1e-9);
    assert_eq!(solid.evidence().occupancy_spans, 4);
}

#[test]
fn tolerance_collapsed_occupancy_is_reported_not_stored() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 8.0), 1.0);
    let thin =
        sample_triangles_cpu(&config, &fixtures::closed_slab(frame, 2.0, 2.0 + 1e-9, 3.0)).unwrap();
    let coarse = Tolerance::new(1e-3, 1e-9).unwrap();
    assert_eq!(
        thin.derive_occupancy(coarse),
        Err(LayeredFieldError::DegenerateOccupancy)
    );
}
