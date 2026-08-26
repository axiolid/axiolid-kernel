use std::ops::ControlFlow;

use axiolid_core::{Aabb, Ray3, Vec3};
use axiolid_spatial::{Bvh, SpatialIndex, SpatialItem};

fn bounds(min: [f64; 3], max: [f64; 3]) -> Aabb {
    Aabb {
        min: Vec3::from_array(min),
        max: Vec3::from_array(max),
    }
}

#[test]
fn distance_pairs_match_brute_force_in_insertion_order() {
    let items: Vec<_> = (0..384_u32)
        .map(|index| {
            let x = ((index * 37) % 29) as f64;
            let y = ((index * 17) % 31) as f64;
            let z = ((index * 11) % 13) as f64;
            SpatialItem::new(index, bounds([x, y, z], [x + 0.8, y + 1.2, z + 0.5]))
        })
        .collect();
    let tree = Bvh::build(items.clone());

    let result = tree.pairs_within_distance(1.75);
    let got: Vec<_> = result
        .pairs
        .into_iter()
        .map(|pair| (pair.a, pair.b))
        .collect();
    let mut expected = Vec::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            if items[i].bounds.gap(&items[j].bounds) <= 1.75 {
                expected.push((items[i].key, items[j].key));
            }
        }
    }

    assert_eq!(got, expected);
    assert!(result.stats.tested_items < items.len() * (items.len() - 1) / 2);
}

#[test]
fn visit_aabb_stops_without_allocating_an_output() {
    let tree = Bvh::build([
        SpatialItem::new("first", bounds([0.0; 3], [1.0; 3])),
        SpatialItem::new("second", bounds([2.0, 0.0, 0.0], [3.0, 1.0, 1.0])),
    ]);
    let mut visited = Vec::new();

    tree.visit_aabb(&bounds([-1.0; 3], [4.0; 3]), &mut |key| {
        visited.push(*key);
        ControlFlow::Break(())
    });

    assert_eq!(visited, vec!["first"]);
}

#[test]
fn ray_visitation_is_ascending_and_tie_stable() {
    let tree = Bvh::build([
        SpatialItem::new(10_u32, bounds([1.0, 0.0, 0.0], [2.0, 1.0, 1.0])),
        SpatialItem::new(20, bounds([1.0, 0.2, 0.0], [2.0, 0.8, 1.0])),
        SpatialItem::new(30, bounds([5.0, 0.0, 0.0], [6.0, 1.0, 1.0])),
    ]);
    let ray = Ray3 {
        origin: Vec3::new(0.0, 0.5, 0.5),
        direction: Vec3::X,
    };
    let mut hits = Vec::new();

    tree.visit_ray(&ray, &mut |hit| {
        hits.push((*hit.key, hit.distance));
        ControlFlow::Continue(())
    });

    assert_eq!(hits, vec![(10, 1.0), (20, 1.0), (30, 5.0)]);
}

#[test]
fn malformed_bounds_are_rejected_observably() {
    let tree = Bvh::build([
        SpatialItem::new(1_u32, bounds([0.0; 3], [1.0; 3])),
        SpatialItem::new(2, Aabb::empty()),
        SpatialItem::new(3, bounds([f64::NAN, 0.0, 0.0], [1.0, 1.0, 1.0])),
    ]);

    assert_eq!(tree.len(), 1);
    assert_eq!(tree.rejected_items(), 2);
}
