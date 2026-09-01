//! Shared multi-layer fixtures for the field contract tests.
//!
//! Each integration test binary uses a different subset, so unused helpers are
//! expected per binary rather than dead code.
#![allow(dead_code)]

use axiolid_core::{Frame3, Tolerance, Vec3};
use axiolid_field::{FieldBounds, FieldConfig, FieldResourceBudget, Triangle3};

/// World-aligned frame: local z is world +Z.
pub fn z_up_frame() -> Frame3 {
    Frame3 {
        origin: Vec3::ZERO,
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
    }
}

/// Deliberately non-Z-up right-handed frame: local z is world +X, and the
/// origin is offset so a bug that assumes world coordinates cannot pass.
pub fn x_up_frame() -> Frame3 {
    Frame3 {
        origin: Vec3::new(10.0, -4.0, 3.0),
        x: Vec3::Y,
        y: Vec3::Z,
        z: Vec3::X,
    }
}

pub fn config(frame: Frame3, max: Vec3, cell_size: f64) -> FieldConfig {
    FieldConfig::new(
        frame,
        FieldBounds::new(Vec3::ZERO, max).unwrap(),
        cell_size,
        Tolerance::METRE,
        FieldResourceBudget::new(4096, 65536),
    )
    .unwrap()
}

/// An axis-aligned quad at local height `w` covering the local x/y square
/// `[-1, extent]`, expressed in the frame's own space.
pub fn quad_at(frame: Frame3, w: f64, extent: f64) -> [Triangle3; 2] {
    let point = |u: f64, v: f64| frame.origin + frame.x * u + frame.y * v + frame.z * w;
    let a = point(-1.0, -1.0);
    let b = point(extent, -1.0);
    let c = point(extent, extent);
    let d = point(-1.0, extent);
    [Triangle3::new(a, b, c), Triangle3::new(a, c, d)]
}

/// Three stacked slabs in one field: a genuine multi-layer fixture that a
/// single-floor raster cannot represent.
pub fn three_stacked_slabs(frame: Frame3, extent: f64) -> Vec<Triangle3> {
    let mut out = Vec::new();
    for w in [1.0, 3.0, 5.0] {
        out.extend(quad_at(frame, w, extent));
    }
    out
}

/// A closed slab spanning `[lo, hi]` on local z, wound so bottom facets face
/// against local +z and top facets face with it.
pub fn closed_slab(frame: Frame3, lo: f64, hi: f64, extent: f64) -> Vec<Triangle3> {
    let point = |u: f64, v: f64, w: f64| frame.origin + frame.x * u + frame.y * v + frame.z * w;
    let (l, h) = (-1.0_f64, extent);
    let corners = |w: f64| {
        [
            point(l, l, w),
            point(h, l, w),
            point(h, h, w),
            point(l, h, w),
        ]
    };
    let bottom = corners(lo);
    let top = corners(hi);
    vec![
        Triangle3::new(bottom[0], bottom[2], bottom[1]),
        Triangle3::new(bottom[0], bottom[3], bottom[2]),
        Triangle3::new(top[0], top[1], top[2]),
        Triangle3::new(top[0], top[2], top[3]),
    ]
}
