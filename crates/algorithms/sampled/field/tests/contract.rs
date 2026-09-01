//! Representation and configuration contract for the layered field.

mod fixtures;

use axiolid_core::{Frame3, Interval, Tolerance, Vec3};
use axiolid_field_ops::{
    FieldBounds, FieldConfig, FieldResourceBudget, LayeredCell, LayeredField, LayeredFieldError,
    SurfaceFacing, SurfaceHit,
};

#[test]
fn cell_may_hold_zero_one_or_many_layers() {
    let empty = LayeredCell::empty();
    assert!(empty.is_empty());
    assert_eq!(empty.surfaces().len(), 0);
    assert_eq!(empty.occupancy().len(), 0);

    let single = LayeredCell::new(vec![Interval::new(1.0, 2.0)]).unwrap();
    assert_eq!(single.occupancy().len(), 1);

    let many = LayeredCell::new(vec![
        Interval::new(7.0, 8.0),
        Interval::new(1.0, 2.0),
        Interval::new(4.0, 5.0),
    ])
    .unwrap();
    assert_eq!(
        many.occupancy(),
        &[
            Interval::new(1.0, 2.0),
            Interval::new(4.0, 5.0),
            Interval::new(7.0, 8.0)
        ]
    );
    assert_eq!(many.layer_count(), 3);
}

#[test]
fn surfaces_and_occupancy_are_separate_channels() {
    let cell = LayeredCell::with_layers(
        vec![
            SurfaceHit::new(4.0, SurfaceFacing::WithNormal),
            SurfaceHit::new(1.0, SurfaceFacing::AgainstNormal),
        ],
        vec![Interval::new(2.0, 3.0)],
    )
    .unwrap();
    assert_eq!(cell.surfaces()[0].w(), 1.0);
    assert_eq!(cell.surfaces()[1].w(), 4.0);
    assert_eq!(cell.occupancy(), &[Interval::new(2.0, 3.0)]);
    assert_eq!(cell.layer_count(), 3);
}

#[test]
fn equal_coordinate_surface_ties_are_deterministic() {
    let cell = LayeredCell::with_layers(
        vec![
            SurfaceHit::new(2.0, SurfaceFacing::WithNormal),
            SurfaceHit::new(2.0, SurfaceFacing::AgainstNormal),
        ],
        Vec::new(),
    )
    .unwrap();
    assert_eq!(cell.surfaces()[0].facing(), SurfaceFacing::AgainstNormal);
    assert_eq!(cell.surfaces()[1].facing(), SurfaceFacing::WithNormal);
}

#[test]
fn touching_or_overlapping_occupancy_is_not_silently_repaired() {
    assert_eq!(
        LayeredCell::new(vec![Interval::new(1.0, 2.0), Interval::new(2.0, 3.0)]),
        Err(LayeredFieldError::NonDisjointIntervals)
    );
    assert_eq!(
        LayeredCell::new(vec![Interval::new(1.0, 3.0), Interval::new(2.0, 4.0)]),
        Err(LayeredFieldError::NonDisjointIntervals)
    );
    assert_eq!(
        LayeredCell::new(vec![Interval::new(2.0, 2.0)]),
        Err(LayeredFieldError::InvalidInterval)
    );
    assert_eq!(
        LayeredCell::new(vec![Interval::new(f64::NAN, 1.0)]),
        Err(LayeredFieldError::InvalidInterval)
    );
}

#[test]
fn grid_uses_deterministic_row_major_addresses() {
    let field = LayeredField::empty(2, 3).unwrap();
    assert_eq!(field.cell_count(), 6);
    assert_eq!(field.dimensions(), (2, 3));
    assert_eq!(field.linear_index(1, 2), Some(5));
    assert_eq!(field.linear_index(2, 0), None);
    assert!(field.cell(1, 2).is_some());
    assert!(field.cell(2, 2).is_none());
}

#[test]
fn configuration_is_explicit_and_frame_neutral() {
    let config = fixtures::config(fixtures::x_up_frame(), Vec3::new(2.0, 3.0, 5.0), 1.0);
    assert_eq!(config.dimensions(), (2, 3));
    assert_eq!(config.cell_size(), 1.0);
    assert_eq!(config.tolerance(), Tolerance::METRE);
    assert_eq!(LayeredField::with_config(&config).unwrap().cell_count(), 6);
}

#[test]
fn cell_centers_follow_the_frame_not_the_world_axes() {
    let frame = fixtures::x_up_frame();
    let config = fixtures::config(frame, Vec3::new(2.0, 2.0, 4.0), 1.0);
    // Local (0,0) centre is half a cell along local x and y from the origin.
    let expected = frame.origin + frame.x * 0.5 + frame.y * 0.5;
    let center = config.cell_center(0, 0);
    assert!((center - expected).length() < 1e-12, "{center:?}");
    // A second cell steps along the frame's local x, i.e. world +Y here.
    let stepped = config.cell_center(1, 0);
    assert!(
        (stepped - (expected + frame.x)).length() < 1e-12,
        "{stepped:?}"
    );
}

#[test]
fn configuration_rejects_invalid_frames_and_bounds() {
    let budget = FieldResourceBudget::new(64, 64);
    let degenerate = Frame3 {
        origin: Vec3::ZERO,
        x: Vec3::X,
        y: Vec3::X,
        z: Vec3::Z,
    };
    assert_eq!(
        FieldConfig::new(
            degenerate,
            FieldBounds::new(Vec3::ZERO, Vec3::splat(1.0)).unwrap(),
            1.0,
            Tolerance::METRE,
            budget
        ),
        Err(LayeredFieldError::InvalidFrame)
    );

    let left_handed = Frame3 {
        origin: Vec3::ZERO,
        x: Vec3::X,
        y: Vec3::Y,
        z: -Vec3::Z,
    };
    assert_eq!(
        FieldConfig::new(
            left_handed,
            FieldBounds::new(Vec3::ZERO, Vec3::splat(1.0)).unwrap(),
            1.0,
            Tolerance::METRE,
            budget
        ),
        Err(LayeredFieldError::InvalidFrame)
    );

    assert_eq!(
        FieldBounds::new(Vec3::splat(1.0), Vec3::ZERO),
        Err(LayeredFieldError::InvalidBounds)
    );
    assert_eq!(
        FieldBounds::new(Vec3::ZERO, Vec3::new(f64::INFINITY, 1.0, 1.0)),
        Err(LayeredFieldError::InvalidBounds)
    );

    assert_eq!(
        FieldConfig::new(
            fixtures::z_up_frame(),
            FieldBounds::new(Vec3::ZERO, Vec3::splat(1.0)).unwrap(),
            0.0,
            Tolerance::METRE,
            budget
        ),
        Err(LayeredFieldError::InvalidCellSize)
    );
}

#[test]
fn configuration_refuses_budget_exhaustion() {
    assert_eq!(
        FieldConfig::new(
            fixtures::z_up_frame(),
            FieldBounds::new(Vec3::ZERO, Vec3::new(3.0, 3.0, 1.0)).unwrap(),
            1.0,
            Tolerance::METRE,
            FieldResourceBudget::new(8, 8),
        ),
        Err(LayeredFieldError::CellBudgetExceeded)
    );
}
