//! Tessellation of CSG primitives into closed solids.
//!
//! Every variant is analytic, so each is emitted directly. A block has no
//! curvature to sample; a sphere's is exactly known. The chord budget still
//! decides the radial segment count so a primitive and a swept face of the
//! same radius agree on what a tolerance means.

use axiolid_core::{Point3, Scalar, Tolerance};
use axiolid_kernel::{GeomError, GeomResult};
use axiolid_mesh::TriMesh;
use axiolid_primitive::Primitive;

/// Radial segments needed for `radius` under a chord budget.
///
/// Same sagitta rule the curve flattener uses: r(1 - cos(pi/n)) <= tol.
/// Clamped low so a degenerate tolerance cannot ask for an unbounded mesh.
fn segments(radius: Scalar, tolerance: Scalar) -> usize {
    if !(radius.is_finite() && radius > 0.0 && tolerance.is_finite() && tolerance > 0.0) {
        return 3;
    }
    let ratio = 1.0 - (tolerance / radius).min(1.0);
    let n = (core::f64::consts::PI / ratio.acos().max(1e-9)).ceil();
    (n as usize).clamp(3, 4096)
}

/// Tessellate one CSG primitive into a closed, outward-wound solid.
///
/// Outward winding is not decoration: `axiolid-boolmesh` and the clash
/// containment test both read signed volume, and an inverted primitive
/// silently produces negative volume and wrong verdicts.
pub fn tessellate_primitive(primitive: &Primitive, tolerance: Tolerance) -> GeomResult<TriMesh> {
    let tol = tolerance.linear();
    match primitive {
        Primitive::Block { x, y, z } => block(*x, *y, *z),
        Primitive::Sphere { radius } => sphere(*radius, tol),
        Primitive::Cylinder { radius, height } => cylinder(*radius, *height, tol),
        Primitive::Cone { radius, height } => cone(*radius, *height, tol),
        Primitive::Pyramid { x, y, height } => pyramid(*x, *y, *height),
        // The enum is non_exhaustive: a new family is unsupported, never
        // silently approximated by the nearest one.
        _ => Err(GeomError::Unsupported {
            backend: crate::ScalarBoolean::ID,
            operation: axiolid_kernel::Operation::Tessellation,
        }),
    }
}

/// Validate a positive finite extent, naming the offender.
fn positive(value: Scalar, what: &str) -> GeomResult<Scalar> {
    if !value.is_finite() || value <= 0.0 {
        return Err(GeomError::InvalidInput(format!(
            "{what} must be positive and finite, got {value}"
        )));
    }
    Ok(value)
}

/// Axis-aligned block centred on the local origin.
fn block(x: Scalar, y: Scalar, z: Scalar) -> GeomResult<TriMesh> {
    let (hx, hy, hz) = (
        positive(x, "block x")? / 2.0,
        positive(y, "block y")? / 2.0,
        positive(z, "block z")? / 2.0,
    );
    let p = vec![
        Point3::new(-hx, -hy, -hz),
        Point3::new(hx, -hy, -hz),
        Point3::new(hx, hy, -hz),
        Point3::new(-hx, hy, -hz),
        Point3::new(-hx, -hy, hz),
        Point3::new(hx, -hy, hz),
        Point3::new(hx, hy, hz),
        Point3::new(-hx, hy, hz),
    ];
    // Outward winding, verified by the positive-volume test rather than by
    // reading the index list.
    let i = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6,
        3, 0, 4, 3, 4, 7,
    ];
    Ok(TriMesh::new(p, i))
}

/// Rectangular pyramid: base on z = 0, apex on +z.
fn pyramid(x: Scalar, y: Scalar, height: Scalar) -> GeomResult<TriMesh> {
    let (hx, hy) = (
        positive(x, "pyramid x")? / 2.0,
        positive(y, "pyramid y")? / 2.0,
    );
    let h = positive(height, "pyramid height")?;
    let p = vec![
        Point3::new(-hx, -hy, 0.0),
        Point3::new(hx, -hy, 0.0),
        Point3::new(hx, hy, 0.0),
        Point3::new(-hx, hy, 0.0),
        Point3::new(0.0, 0.0, h),
    ];
    let i = vec![0, 2, 1, 0, 3, 2, 0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4];
    Ok(TriMesh::new(p, i))
}

/// A ring of `n` points at `radius`, height `z`.
fn ring(radius: Scalar, z: Scalar, n: usize) -> Vec<Point3> {
    (0..n)
        .map(|k| {
            let a = core::f64::consts::TAU * (k as Scalar) / (n as Scalar);
            Point3::new(radius * a.cos(), radius * a.sin(), z)
        })
        .collect()
}

/// Cylinder along +z, base on z = 0.
fn cylinder(radius: Scalar, height: Scalar, tol: Scalar) -> GeomResult<TriMesh> {
    let r = positive(radius, "cylinder radius")?;
    let h = positive(height, "cylinder height")?;
    let n = segments(r, tol);
    let mut p = ring(r, 0.0, n);
    p.extend(ring(r, h, n));
    p.push(Point3::new(0.0, 0.0, 0.0));
    p.push(Point3::new(0.0, 0.0, h));
    let (bc, tc) = (2 * n, 2 * n + 1);
    let mut i = Vec::with_capacity(n * 12);
    for k in 0..n {
        let (a, b) = (k, (k + 1) % n);
        // Side quad, then the two caps. The base fan is wound opposite to
        // the top so both face away from the enclosed volume.
        i.extend([a as u32, b as u32, (b + n) as u32]);
        i.extend([a as u32, (b + n) as u32, (a + n) as u32]);
        i.extend([bc as u32, b as u32, a as u32]);
        i.extend([tc as u32, (a + n) as u32, (b + n) as u32]);
    }
    Ok(TriMesh::new(p, i))
}

/// Cone along +z: base ring on z = 0, apex at height.
fn cone(radius: Scalar, height: Scalar, tol: Scalar) -> GeomResult<TriMesh> {
    let r = positive(radius, "cone radius")?;
    let h = positive(height, "cone height")?;
    let n = segments(r, tol);
    let mut p = ring(r, 0.0, n);
    p.push(Point3::new(0.0, 0.0, 0.0));
    p.push(Point3::new(0.0, 0.0, h));
    let (base, apex) = (n, n + 1);
    let mut i = Vec::with_capacity(n * 6);
    for k in 0..n {
        let (a, b) = (k as u32, ((k + 1) % n) as u32);
        i.extend([base as u32, b, a]);
        i.extend([a, b, apex as u32]);
    }
    Ok(TriMesh::new(p, i))
}

/// Sphere centred on the local origin, as a UV mesh.
fn sphere(radius: Scalar, tol: Scalar) -> GeomResult<TriMesh> {
    let r = positive(radius, "sphere radius")?;
    let n = segments(r, tol);
    // Half as many stacks as segments: the polar direction spans PI, not TAU,
    // so equal counts would oversample it by 2x for the same chord error.
    let stacks = (n / 2).max(2);
    let mut p = Vec::with_capacity((stacks + 1) * n);
    for i in 0..=stacks {
        let v = core::f64::consts::PI * (i as Scalar) / (stacks as Scalar);
        for j in 0..n {
            let u = core::f64::consts::TAU * (j as Scalar) / (n as Scalar);
            p.push(Point3::new(
                r * v.sin() * u.cos(),
                r * v.sin() * u.sin(),
                r * v.cos(),
            ));
        }
    }
    let mut idx = Vec::with_capacity(stacks * n * 6);
    for i in 0..stacks {
        for j in 0..n {
            let jn = (j + 1) % n;
            // A pole row's n entries are the same point, so they must map
            // to ONE index or no triangle around the pole shares an edge.
            let row = |r: usize, k: usize| -> u32 {
                if r == 0 || r == stacks {
                    (r * n) as u32
                } else {
                    (r * n + k) as u32
                }
            };
            let a = row(i, j);
            let b = row(i, jn);
            let c = row(i + 1, j);
            let d = row(i + 1, jn);
            // Pole rows collapse to a point; skip the degenerate half.
            if i > 0 {
                idx.extend([a, c, b]);
            }
            if i + 1 < stacks {
                idx.extend([b, c, d]);
            }
        }
    }
    Ok(TriMesh::new(p, idx))
}
