use axiolid_core::{Aabb, Point3, Tolerance};
use axiolid_measure::surface_properties;
use axiolid_mesh::TriMesh;
use axiolid_spatial::{Bvh, SpatialItem};

fn main() {
    // Exercise the capability, not just the type name: a fixture that only
    // mentions a symbol can pass while the package is unusable.
    let mesh = TriMesh::new(
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        vec![0, 1, 2],
    );
    let props = surface_properties(&mesh, Tolerance::METRE).expect("surface properties");
    assert!((props.area - 0.5).abs() < 1e-12, "unit triangle area");

    let mut bounds = Aabb::from_point(Point3::new(0.0, 0.0, 0.0));
    bounds.extend(Point3::new(1.0, 1.0, 1.0));
    let bvh = Bvh::build([SpatialItem::new(7u32, bounds)]);
    assert_eq!(bvh.len(), 1, "one accepted broad-phase item");

    println!("mesh-rule-checker ok");
}
