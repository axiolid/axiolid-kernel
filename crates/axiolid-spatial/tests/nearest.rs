use axiolid_core::{Aabb, Vec3};
use axiolid_spatial::{Bvh, SpatialItem};

fn bounds(min: [f64; 3], max: [f64; 3]) -> Aabb {
    Aabb {
        min: Vec3::from_array(min),
        max: Vec3::from_array(max),
    }
}

#[test]
fn nearest_query_is_stable_and_prunes_leaves() {
    let tree = Bvh::build((0..128_u32).map(|index| {
        let x = index as f64 * 10.0;
        SpatialItem::new(index, bounds([x, 0.0, 0.0], [x + 1.0, 1.0, 1.0]))
    }));

    let nearest = tree
        .nearest_to(&bounds([12.0, 0.0, 0.0], [13.0, 1.0, 1.0]), |_| true)
        .expect("tree has accepted candidates");

    assert_eq!(nearest.key, 1);
    assert_eq!(nearest.lower_bound, 1.0);
    assert!(nearest.stats.tested_items < tree.len());
}

#[test]
fn nearest_rejects_malformed_query_bounds() {
    let tree = Bvh::build([SpatialItem::new(1_u32, bounds([0.0; 3], [1.0; 3]))]);
    assert!(std::panic::catch_unwind(|| tree.nearest_to(&Aabb::empty(), |_| true)).is_err());
}
