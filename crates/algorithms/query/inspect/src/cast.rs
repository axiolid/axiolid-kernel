//! First surface hit along a ray.
//!
//! Which triangle is hit is decided by the same exact `orient3d` parity the
//! containment query uses, so a caller cannot get "the ray hits nothing"
//! from one and "inside" from the other. Only the hit PARAMETER is computed
//! in f64, and it is a reported measurement rather than a decision input
//! (ADR 0045).

use crate::containment::ray_hits_triangle;
use axiolid_core::{Point3, Vec3};
use axiolid_mesh::TriMesh;

/// Where a ray first meets a mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// Index of the triangle struck.
    pub triangle: usize,
    /// Ray parameter of the hit: `origin + direction * t`.
    pub t: f64,
    /// The hit position.
    pub point: Point3,
}

/// First triangle the ray from `origin` along `direction` strikes.
///
/// Returns `None` when the ray misses the mesh entirely, or when it meets an
/// edge or vertex exactly -- the degenerate case the exact test refuses to
/// resolve arbitrarily rather than picking a neighbour at random.
pub fn ray_cast(mesh: &TriMesh, origin: Point3, direction: Vec3) -> Option<Hit> {
    if direction.length_squared() == 0.0 {
        return None;
    }
    let mut nearest: Option<Hit> = None;
    for index in 0..mesh.indices.len() / 3 {
        let Some(triangle) = crate::clearance::triangle_at(mesh, index) else {
            continue;
        };
        // Membership is exact; only the distance along the ray is f64.
        if !ray_hits_triangle(triangle, origin, direction)? {
            continue;
        }
        let Some(t) = ray_parameter(triangle, origin, direction) else {
            continue;
        };
        if t < 0.0 {
            continue;
        }
        if nearest.is_none_or(|hit| t < hit.t) {
            nearest = Some(Hit {
                triangle: index,
                t,
                point: origin + direction * t,
            });
        }
    }
    nearest
}

/// Ray parameter where the triangle's plane is met.
///
/// Membership has already been decided exactly by the caller; this only
/// measures how far along. `None` when the ray runs parallel to the plane.
fn ray_parameter(triangle: [Point3; 3], origin: Point3, direction: Vec3) -> Option<f64> {
    let [a, b, c] = triangle;
    let normal = (b - a).cross(c - a);
    let denominator = normal.dot(direction);
    if denominator == 0.0 {
        return None;
    }
    let t = normal.dot(a - origin) / denominator;
    t.is_finite().then_some(t)
}
