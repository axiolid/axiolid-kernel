//! The facade actually re-exports the planar project and route capabilities.
//!
//! A feature that compiles but exposes nothing is indistinguishable from a
//! missing feature at the call site, so name real items rather than just
//! enabling the flags.

#![cfg(all(feature = "project", feature = "route"))]

use axiolid::core::{Point2, Tolerance};

#[test]
fn the_route_module_is_reachable_through_the_facade() {
    // A square region with no barriers: the shortest path is the straight
    // segment, so this asserts the re-export carries real behaviour.
    let square = vec![
        Point2::new(0.0, 0.0),
        Point2::new(4.0, 0.0),
        Point2::new(4.0, 4.0),
        Point2::new(0.0, 4.0),
    ];
    let region = vec![axiolid::overlay::Polygon {
        outer: axiolid::overlay::Ring { points: square },
        holes: Vec::new(),
    }];
    let found =
        axiolid::route::shortest_path(&region, &[], Point2::new(1.0, 1.0), Point2::new(3.0, 3.0));
    let route = found
        .expect("query is well formed")
        .expect("a route exists");
    assert!(route.length > 0.0);
}

#[test]
fn the_project_module_is_reachable_through_the_facade() {
    // Project a unit triangle onto the ground plane: a real call through the
    // facade path, not just a type mention.
    let mesh = axiolid::mesh::TriMesh::new(
        vec![
            axiolid::core::Point3::new(0.0, 0.0, 1.0),
            axiolid::core::Point3::new(1.0, 0.0, 1.0),
            axiolid::core::Point3::new(0.0, 1.0, 1.0),
        ],
        vec![0, 1, 2],
    );
    let tol = Tolerance::new(1e-6, 1e-9).expect("tolerance");
    let projection = axiolid::project::project_mesh(&mesh, axiolid::project::Plane::ground(), tol)
        .expect("a unit triangle projects");
    assert!(!projection.polygons.is_empty());
}
