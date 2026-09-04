//! Analytic cellular subtraction: axis-aligned box host minus axis-aligned box
//! cutters, without running a general mesh boolean.
//!
//! This is a prototype of the optimisation ifc-lite calls `rect_fast`. The
//! insight is that the dominant IFC case -- a wall (axis-aligned box) with
//! rectangular openings (axis-aligned boxes) -- has an exact closed-form
//! answer, so the general intersection machinery is pure overhead there.
//!
//! Method: collect every cutter face plane as a grid line on each axis, which
//! partitions the host into a 3D grid of cells. Each cell is entirely solid or
//! entirely void (no cutter boundary can pass through a cell interior, because
//! every cutter boundary IS a grid plane). Emit a face wherever a solid cell
//! meets a void cell or the grid boundary.
//!
//! Watertight by construction: adjacent cells address shared grid vertices by
//! integer index, so coincident corners are bit-identical and no T-junctions
//! arise.
//!
//! # Determinism
//!
//! Vertex identity is a `BTreeMap` keyed by integer grid index, NOT a
//! `HashMap`. `std`'s `RandomState` seeds each map instance differently, which
//! is the root cause of the nondeterminism found in upstream `boolmesh`, whose
//! vertex dedup uses a randomly-seeded `HashMap`. Ordered keys make this path's
//! output byte-reproducible across processes.
//!
//! # Preconditions
//!
//! Host and cutters must be axis-aligned boxes. [`crate::box_detect::recognise`]
//! establishes that structurally before this module runs; this module trusts
//! that guarantee and does not re-derive it.
//!
//! # Opt-in only
//!
//! Never auto-dispatched. The caller selects this path explicitly, so a run's
//! output topology is predictable: the same input always takes the same route.
//! Silently switching strategies based on input shape would make triangle
//! counts and vertex ordering vary for reasons a caller cannot see.

use axiolid_core::Point3;
use axiolid_mesh::TriMesh;
use std::collections::BTreeMap;

use crate::box_detect::AlignedBox;

/// Inclusive-exclusive interval grid on one axis.
struct Axis {
    coords: Vec<f64>,
}

impl Axis {
    /// Host range plus every cutter boundary that falls strictly inside it.
    ///
    /// Cutter planes outside the host contribute nothing: the host clips them.
    /// De-duplicated with a tolerance so a cutter flush against the host face
    /// (a door at floor level) does not create a zero-thickness cell.
    fn build(lo: f64, hi: f64, cuts: impl Iterator<Item = f64>, eps: f64) -> Self {
        let mut coords = vec![lo, hi];
        for c in cuts {
            if c > lo + eps && c < hi - eps {
                coords.push(c);
            }
        }
        coords.sort_by(|a, b| a.partial_cmp(b).expect("finite coordinates"));
        coords.dedup_by(|a, b| (*a - *b).abs() <= eps);
        Self { coords }
    }

    fn cells(&self) -> usize {
        self.coords.len() - 1
    }

    fn mid(&self, i: usize) -> f64 {
        0.5 * (self.coords[i] + self.coords[i + 1])
    }
}

/// Subtract axis-aligned `cutters` from axis-aligned `host` analytically.
///
/// Returns `None` when the problem is outside this path's competence, so the
/// caller can fall back to the exact kernel rather than receive a wrong answer:
/// * no cutters (nothing to do -- the caller should skip the boolean entirely)
/// * a cell count above `max_cells` (the grid is `O(n^3)` in cutter count, so a
///   pathological input must not be allowed to explode)
pub fn subtract_boxes(
    host: &AlignedBox,
    cutters: &[AlignedBox],
    max_cells: usize,
) -> Option<TriMesh> {
    if cutters.is_empty() {
        return None;
    }
    let (lo, hi) = (host.min, host.max);
    // Scale-aware: two planes closer than this would collapse under f64
    // cancellation at building coordinates, cracking the result.
    let span = (0..3).map(|a| hi[a] - lo[a]).fold(0.0, f64::max);
    let eps = span * 1e-12;

    // Only cutters that actually overlap the host contribute grid planes.
    let overlapping: Vec<&AlignedBox> = cutters
        .iter()
        .filter(|c| (0..3).all(|a| c.max[a] > lo[a] + eps && c.min[a] < hi[a] - eps))
        .collect();
    if overlapping.is_empty() {
        return None;
    }

    let axes: Vec<Axis> = (0..3)
        .map(|a| {
            Axis::build(
                lo[a],
                hi[a],
                overlapping.iter().flat_map(|c| [c.min[a], c.max[a]]),
                eps,
            )
        })
        .collect();

    let (nx, ny, nz) = (axes[0].cells(), axes[1].cells(), axes[2].cells());
    if nx.saturating_mul(ny).saturating_mul(nz) > max_cells {
        return None;
    }

    // solid[i][j][k]: cell centre lies in the host and outside every cutter.
    // The centre decides the whole cell because no cutter face crosses a cell
    // interior -- every cutter face is a grid plane.
    let mut solid = vec![false; nx * ny * nz];
    let at = |i: usize, j: usize, k: usize| (i * ny + j) * nz + k;
    for i in 0..nx {
        let cx = axes[0].mid(i);
        for j in 0..ny {
            let cy = axes[1].mid(j);
            for k in 0..nz {
                let c = [cx, cy, axes[2].mid(k)];
                let inside_cutter = overlapping
                    .iter()
                    .any(|b| (0..3).all(|a| c[a] > b.min[a] && c[a] < b.max[a]));
                solid[at(i, j, k)] = !inside_cutter;
            }
        }
    }

    // Shared vertex identity by integer grid index -> bit-identical corners.
    let mut ids: BTreeMap<(usize, usize, usize), u32> = BTreeMap::new();
    let mut positions: Vec<Point3> = Vec::new();
    let mut vertex = |g: (usize, usize, usize), positions: &mut Vec<Point3>| -> u32 {
        if let Some(v) = ids.get(&g) {
            return *v;
        }
        let v = positions.len() as u32;
        positions.push(Point3::new(
            axes[0].coords[g.0],
            axes[1].coords[g.1],
            axes[2].coords[g.2],
        ));
        ids.insert(g, v);
        v
    };

    let mut indices: Vec<u32> = Vec::new();
    // For axis `a`, (b, c) is the cyclic pair making (a, b, c) right-handed, so
    // u x v along +a. That fixes outward winding without per-face sign fixes.
    const CYCLE: [(usize, usize); 3] = [(1, 2), (2, 0), (0, 1)];

    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                if !solid[at(i, j, k)] {
                    continue;
                }
                let cell = [i, j, k];
                let dims = [nx, ny, nz];
                for a in 0..3 {
                    let (b, c) = CYCLE[a];
                    for &positive in &[true, false] {
                        // Emit only where this solid cell meets void or the edge
                        // of the grid: interior shared faces cancel.
                        let neighbour_solid = if positive {
                            cell[a] + 1 < dims[a] && {
                                let mut n = cell;
                                n[a] += 1;
                                solid[at(n[0], n[1], n[2])]
                            }
                        } else {
                            cell[a] > 0 && {
                                let mut n = cell;
                                n[a] -= 1;
                                solid[at(n[0], n[1], n[2])]
                            }
                        };
                        if neighbour_solid {
                            continue;
                        }
                        let plane = if positive { cell[a] + 1 } else { cell[a] };
                        let mut g = [0usize; 3];
                        g[a] = plane;
                        let corner = |db: usize, dc: usize, g: &[usize; 3]| {
                            let mut q = *g;
                            q[b] = cell[b] + db;
                            q[c] = cell[c] + dc;
                            (q[0], q[1], q[2])
                        };
                        let p00 = vertex(corner(0, 0, &g), &mut positions);
                        let p10 = vertex(corner(1, 0, &g), &mut positions);
                        let p11 = vertex(corner(1, 1, &g), &mut positions);
                        let p01 = vertex(corner(0, 1, &g), &mut positions);
                        if positive {
                            indices.extend_from_slice(&[p00, p10, p11, p00, p11, p01]);
                        } else {
                            indices.extend_from_slice(&[p00, p11, p10, p00, p01, p11]);
                        }
                    }
                }
            }
        }
    }

    Some(TriMesh::new(positions, indices))
}
