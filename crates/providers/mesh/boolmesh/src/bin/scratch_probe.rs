//! Measure `boolmesh` peak scratch so its declared bound is evidence-based.
//!
//! ADR 0017 section 4 says `ScratchRequirement::Unbounded` is a declared
//! deficiency, not a resting state. Replacing it requires a *measured* bound,
//! not a guessed one, so this bench wraps the global allocator with a counter
//! and reports peak bytes per input triangle across representative workloads.
//!
//! Run with:
//! ```text
//! cargo run --release -p axiolid-mesh-boolean-boolmesh --bin scratch_probe --all-features
//! ```

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use axiolid_core::{BooleanOperator, Point3, Tolerance};
use axiolid_kernel::{ExecutionOptions, MeshBoolean};
use axiolid_mesh::TriMesh;

use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// Counting allocator: tracks live bytes and the high-water mark.
struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn box_at(min: [f64; 3], max: [f64; 3]) -> TriMesh {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let positions: Vec<Point3> = vec![
        [x0, y0, z0].into(),
        [x1, y0, z0].into(),
        [x1, y1, z0].into(),
        [x0, y1, z0].into(),
        [x0, y0, z1].into(),
        [x1, y0, z1].into(),
        [x1, y1, z1].into(),
        [x0, y1, z1].into(),
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6,
        3, 0, 4, 3, 4, 7,
    ];
    TriMesh::new(positions, indices)
}

/// Subdivide each triangle four ways, `levels` times, to grow triangle count.
fn subdivide(mesh: &TriMesh, levels: usize) -> TriMesh {
    let mut current = mesh.clone();
    for _ in 0..levels {
        let mut positions = current.positions.clone();
        let mut indices = Vec::new();
        for triangle in current.indices.chunks_exact(3) {
            let [ia, ib, ic] = [triangle[0], triangle[1], triangle[2]];
            let (a, b, c) = (
                current.positions[ia as usize],
                current.positions[ib as usize],
                current.positions[ic as usize],
            );
            let base = positions.len() as u32;
            positions.push((a + b) * 0.5);
            positions.push((b + c) * 0.5);
            positions.push((c + a) * 0.5);
            let (ab, bc, ca) = (base, base + 1, base + 2);
            indices.extend_from_slice(&[ia, ab, ca]);
            indices.extend_from_slice(&[ab, ib, bc]);
            indices.extend_from_slice(&[ca, bc, ic]);
            indices.extend_from_slice(&[ab, bc, ca]);
        }
        current = TriMesh::new(positions, indices);
    }
    current
}

fn main() {
    let provider = BoolmeshBoolean::new();
    let options = ExecutionOptions::new(Tolerance::METRE);

    println!(
        "{:>10}  {:>14}  {:>18}",
        "triangles", "peak bytes", "bytes/triangle"
    );
    let mut worst_per_triangle = 0usize;

    for levels in 0..=3 {
        let subject = subdivide(&box_at([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]), levels);
        let tool = subdivide(&box_at([0.5, 0.5, 0.5], [1.5, 1.5, 1.5]), levels);
        let elements = subject.triangle_count() + tool.triangle_count();

        for operation in BooleanOperator::ALL {
            LIVE.store(0, Ordering::Relaxed);
            PEAK.store(0, Ordering::Relaxed);
            let outcome = provider.boolean(&subject, &tool, operation, &options);
            let peak = PEAK.load(Ordering::Relaxed);
            assert!(outcome.is_ok(), "{operation:?} failed at level {levels}");

            let per_triangle = peak / elements.max(1);
            worst_per_triangle = worst_per_triangle.max(per_triangle);
            println!("{elements:>10}  {peak:>14}  {per_triangle:>18}  {operation:?}");
        }
    }

    println!();
    println!("worst observed bytes/triangle: {worst_per_triangle}");
    println!("Declare PerElement with headroom above this, never below.");
}
