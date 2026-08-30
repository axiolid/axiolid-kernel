//! Revolution of a profile about an axis.
//!
//! The profile is flattened to rings in its own plane, then each ring point
//! is rotated about the axis in steps chosen by the chord budget. A full
//! turn closes on itself; a partial turn is capped by the profile at each
//! end.
//!
//! Pappus gives the oracle: a full revolution has volume `2*pi*R*A`, where
//! `R` is the centroid's distance from the axis and `A` the profile area.

use axiolid_core::{Point2, Point3, Scalar, Tolerance, Vec3};
use axiolid_kernel::{GeomError, GeomResult};
use axiolid_mesh::TriMesh;

use crate::profile::Rings;

/// Rotation of `p` about the axis through `origin` along unit `dir`.
///
/// Rodrigues' formula. Written out rather than pulled from a matrix type
/// so the axis stays arbitrary: a revolution is not a Z-up operation.
fn rotate(p: Point3, origin: Point3, dir: Vec3, angle: Scalar) -> Point3 {
    let v = p - origin;
    let (s, c) = angle.sin_cos();
    origin + v * c + dir.cross(v) * s + dir * (dir.dot(v) * (1.0 - c))
}

/// Angular steps so the swept arc meets the chord budget.
///
/// The widest point of the profile governs: a ring point at distance `r`
/// from the axis traces a circle of that radius, and its sagitta is
/// `r(1 - cos(dtheta/2))`. Using the maximum radius means every other
/// point is sampled at least as finely.
fn steps(max_radius: Scalar, angle: Scalar, tol: Scalar) -> usize {
    if !(max_radius.is_finite() && max_radius > 0.0 && tol.is_finite() && tol > 0.0) {
        return 8;
    }
    let ratio = (1.0 - (tol / max_radius).min(1.0)).clamp(-1.0, 1.0);
    let per = 2.0 * ratio.acos().max(1e-9);
    ((angle.abs() / per).ceil() as usize).clamp(2, 4096)
}

/// Revolve a profile about an axis into a closed solid.
///
/// A full turn wraps its rings by index, so the seam shares vertices by
/// construction rather than by two samplings agreeing numerically. That is
/// the same structure `tessellate_primitive` uses for a cylinder, and it
/// avoids the trim-based path recorded as broken in issue #2.
pub fn revolve(
    rings: &Rings,
    axis_origin: Point3,
    axis_direction: Vec3,
    angle: Scalar,
    tolerance: Tolerance,
) -> GeomResult<TriMesh> {
    if !angle.is_finite() || angle == 0.0 {
        return Err(GeomError::InvalidInput(format!(
            "revolution angle must be finite and non-zero, got {angle}"
        )));
    }
    let len = axis_direction.length();
    if !len.is_finite() || len <= 0.0 {
        return Err(GeomError::InvalidInput(
            "revolution axis must be a finite non-zero direction".to_owned(),
        ));
    }
    let dir = axis_direction / len;
    // The profile lies in the z = 0 plane of its own frame, matching what
    // `profile_rings` produces for extrusion.
    let lift = |p: Point2| Point3::new(p.x, p.y, 0.0);
    let mut loops: Vec<Vec<Point3>> = Vec::with_capacity(1 + rings.holes.len());
    loops.push(rings.outer.iter().map(|p| lift(*p)).collect());
    for hole in &rings.holes {
        loops.push(hole.iter().map(|p| lift(*p)).collect());
    }
    // Widest distance from the axis governs the angular step.
    let mut max_r: Scalar = 0.0;
    for ring in &loops {
        for p in ring {
            let v = *p - axis_origin;
            let radial = v - dir * dir.dot(v);
            max_r = max_r.max(radial.length());
        }
    }
    let full = (angle.abs() - core::f64::consts::TAU).abs() <= 1e-9;
    let n = steps(max_r, angle, tolerance.linear());
    // Stations along the sweep. A full turn emits n distinct ones and wraps;
    // a partial turn emits n + 1 so both ends exist to be capped.
    let stations = if full { n } else { n + 1 };
    let mut positions: Vec<Point3> = Vec::new();
    for s in 0..stations {
        let t = angle * (s as Scalar) / (n as Scalar);
        for ring in &loops {
            for p in ring {
                positions.push(rotate(*p, axis_origin, dir, t));
            }
        }
    }
    let per_station: usize = loops.iter().map(|r| r.len()).sum();
    let mut indices: Vec<u32> = Vec::new();
    // Side walls: each ring edge sweeps into a quad. The station index wraps
    // for a full turn, which is what welds the seam.
    for s in 0..n {
        let a0 = s * per_station;
        let b0 = ((s + 1) % stations) * per_station;
        let mut base = 0usize;
        for (li, ring) in loops.iter().enumerate() {
            let m = ring.len();
            for k in 0..m {
                let kn = (k + 1) % m;
                let (a, b) = ((a0 + base + k) as u32, (a0 + base + kn) as u32);
                let (c, d) = ((b0 + base + k) as u32, (b0 + base + kn) as u32);
                // A hole's wall faces inward, so its winding is reversed.
                if li == 0 {
                    indices.extend([a, d, b, a, c, d]);
                } else {
                    indices.extend([a, b, d, a, d, c]);
                }
            }
            base += m;
        }
    }
    // Caps. A full turn needs none: the wall closes on itself. A partial
    // turn is bounded by the profile at each end, triangulated the same way
    // an extrusion cap is.
    if !full {
        let (points, tris) = crate::profile::triangulate(rings)?;
        debug_assert_eq!(points.len(), per_station);
        let last = n * per_station;
        for t in &tris {
            let (a, b, c) = (t[0], t[1], t[2]);
            indices.extend([a, b, c]);
            indices.extend([last as u32 + a, last as u32 + c, last as u32 + b]);
        }
    }
    Ok(TriMesh::new(positions, indices))
}
