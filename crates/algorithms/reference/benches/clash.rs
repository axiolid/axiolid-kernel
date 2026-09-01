//! Clash scaling: is `interference` usable at model size?
//!
//! Run: `cargo bench -p axiolid-scalar --bench clash`

use std::hint::black_box;
use std::time::Instant;

use axiolid_core::{Point3, Tolerance};
use axiolid_mesh::TriMesh;
use axiolid_scalar::clash::interference;

/// A UV sphere: the cheapest way to get a closed mesh with a tunable
/// triangle count, which is what the scaling question needs.
fn sphere(cx: f64, radius: f64, bands: usize) -> TriMesh {
    let mut positions = Vec::new();
    for i in 0..=bands {
        let v = std::f64::consts::PI * (i as f64) / (bands as f64);
        for j in 0..=bands {
            let u = std::f64::consts::TAU * (j as f64) / (bands as f64);
            positions.push(Point3::new(
                cx + radius * v.sin() * u.cos(),
                radius * v.sin() * u.sin(),
                radius * v.cos(),
            ));
        }
    }
    let row = bands + 1;
    let mut indices = Vec::new();
    for i in 0..bands {
        for j in 0..bands {
            let a = (i * row + j) as u32;
            let b = (i * row + j + 1) as u32;
            let c = ((i + 1) * row + j) as u32;
            let d = ((i + 1) * row + j + 1) as u32;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    TriMesh::new(positions, indices)
}

fn main() {
    println!("clash scaling: two spheres, overlapping\n");
    println!(
        "{:>6} {:>10} {:>12} {:>14} {:>10}",
        "bands", "triangles", "narrow", "elapsed_ms", "us/tri"
    );
    for bands in [8usize, 12, 16, 24, 32, 48] {
        let a = sphere(0.0, 1.0, bands);
        let b = sphere(1.2, 1.0, bands);
        let n = a.indices.len() / 3;
        let start = Instant::now();
        let report = interference(&a, &b, Tolerance::MILLIMETRE).expect("interference");
        let elapsed = start.elapsed();
        black_box(&report);
        // Narrow-phase tests are the real work; rejections are what was pruned.
        let pairs = report.narrow_phase_tests;
        let ms = elapsed.as_secs_f64() * 1e3;
        println!(
            "{:>6} {:>10} {:>12} {:>14.2} {:>10.3}",
            bands,
            n,
            pairs,
            ms,
            ms * 1e3 / (n as f64)
        );
    }
    println!("\nclash scaling: two spheres, DISJOINT (containment path)\n");
    println!(
        "{:>6} {:>10} {:>12} {:>14} {:>10}",
        "bands", "triangles", "narrow", "elapsed_ms", "us/tri"
    );
    for bands in [8usize, 12, 16, 24, 32] {
        let a = sphere(0.0, 1.0, bands);
        let b = sphere(5.0, 1.0, bands);
        let n = a.indices.len() / 3;
        let start = Instant::now();
        let report = interference(&a, &b, Tolerance::MILLIMETRE).expect("interference");
        let elapsed = start.elapsed();
        black_box(&report);
        let ms = elapsed.as_secs_f64() * 1e3;
        println!(
            "{:>6} {:>10} {:>12} {:>14.2} {:>10.3}",
            bands,
            n,
            report.narrow_phase_tests,
            ms,
            ms * 1e3 / (n as f64)
        );
    }
}
