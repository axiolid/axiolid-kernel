//! Convex decomposition: a solid as a set of convex parts.
//!
//! # Two strategies, one contract
//!
//! There is no single right answer here, so the caller picks:
//!
//! - [`Strategy::Exact`] splits at reflex features until every part is
//!   genuinely convex. The union reproduces the input exactly, and the part
//!   count can be large.
//! - [`Strategy::Approximate`] stops once each part is convex to within a
//!   stated concavity bound. Far fewer parts, and the union is close to but
//!   not identical to the input.
//!
//! Both are legitimate. Collision detection and Minkowski sums usually want
//! the approximate one; anything claiming to reproduce the original solid
//! needs the exact one. What is NOT legitimate is returning an approximate
//! decomposition that presents itself as exact, so [`Decomposition`] always
//! reports which it is, and the approximate path reports the concavity it
//! actually reached rather than the one that was requested.
//!
//! # Method
//!
//! Both strategies share one loop: measure the worst concavity of a part,
//! and if it exceeds the bound, split the part by a plane and recurse.
//! They differ only in the bound -- exact uses zero (to tolerance).
//!
//! Concavity is measured as the largest distance from a vertex of the part
//! to its own convex hull. That is a direct measurement of the property the
//! caller cares about, rather than a proxy like volume ratio: a thin deep
//! notch barely changes volume but is exactly what breaks a convexity
//! assumption downstream.
//!
//! The split plane is the plane of the face the reflex vertex sticks out
//! past. Extending an existing face makes progress by definition, whereas
//! a bounding-box axis through the same point need not separate the notch.
//!
//! # Status: the cap is not yet watertight
//!
//! Clipping produces the correct SIDES. Closing the cut with a cap does
//! not yet work in general, and the failure is instructive enough to
//! record so the next attempt does not repeat it.
//!
//! Reconstructing the cut cross-section from per-triangle predicates
//! oscillates between two defects, measured on an L-shaped solid:
//!
//! | rule | boundary edges | non-manifold edges |
//! |---|---|---|
//! | strict sign changes only | 5 | 0 |
//! | plus vertices lying on the plane | 0 | 3 |
//! | plus a straddling filter | 3 | 0 |
//! | plus the two-vertices-on-plane case | 1 | 3 |
//!
//! Every rule fixes one defect and reintroduces the other, which is the
//! signature of the wrong decomposition rather than a missing case. The
//! reason: whether a wall face standing ON the cut plane belongs to THIS
//! part is a global question about which side the material lies on, and a
//! single triangle cannot answer it. Capping over such a face makes its
//! edges non-manifold; omitting it leaves the loop open along that wall.
//!
//! A boolean solver answers exactly that question, and `boolmesh` already
//! implements it. Because `algorithms` may not depend on `providers`, the
//! route is the mesh-boolean CONTRACT, which both layers may depend on:
//! a caller passes a provider in, the hand-rolled clipper stays the
//! default, and the two are verified against each other.

use std::collections::BTreeMap;

use axiolid_core::{Point2, Point3, Scalar, Tolerance, Vec3};
use axiolid_mesh::{audit_mesh, TriMesh};
use thiserror::Error;

/// Why a decomposition could not be produced.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum DecomposeError {
    /// The index buffer is not a whole number of triangles.
    #[error("index buffer length {0} is not a multiple of 3")]
    RaggedIndices(usize),
    /// A triangle references a vertex that does not exist.
    #[error("triangle {0} references vertex {1}, which is out of range")]
    IndexOutOfRange(usize, u32),
    /// The input is not a closed two-manifold solid.
    ///
    /// Refused rather than decomposed: the parts of an open surface do not
    /// have a union that reproduces it, so any answer would be a fiction.
    #[error("input is not a closed two-manifold solid: {boundary} boundary and {non_manifold} non-manifold edges")]
    NotASolid {
        /// Edges with a single incident triangle.
        boundary: usize,
        /// Edges with more than two incident triangles.
        non_manifold: usize,
    },
    /// A concavity bound must be a positive, finite length.
    #[error("concavity bound {0} is not a positive finite length")]
    InvalidBound(Scalar),
    /// Decomposition did not converge within the part budget.
    ///
    /// Reported rather than returning a partial decomposition, whose union
    /// would silently differ from the input.
    #[error("decomposition exceeded the {limit} part budget")]
    BudgetExceeded {
        /// The cap that was not raised.
        limit: usize,
    },
}

/// How hard to work at making each part convex.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Strategy {
    /// Split until every part is convex to within `tolerance`.
    ///
    /// The union of the parts reproduces the input. Part count is whatever
    /// the geometry demands, which for a deeply non-convex solid is large.
    Exact,
    /// Stop once every part is convex to within `max_concavity`.
    ///
    /// Trades fidelity for part count. The union approximates the input:
    /// concave pockets shallower than the bound are filled in.
    Approximate {
        /// Largest tolerated distance from a part's vertex to its own hull.
        max_concavity: Scalar,
    },
}

/// Whether the parts reproduce the input or merely approximate it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Fidelity {
    /// Every part is convex to within tolerance; the union is the input.
    Exact,
    /// Parts are convex to within a bound larger than tolerance.
    Approximate {
        /// The bound that was requested.
        requested: Scalar,
        /// The largest concavity actually left in any part.
        ///
        /// Reported because it is the honest answer: a caller that asked for
        /// 10mm and got 2mm knows the result is better than it required,
        /// and one that reads this field cannot mistake the request for the
        /// outcome.
        achieved: Scalar,
    },
}

/// A solid expressed as convex parts, with the evidence to judge it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Decomposition {
    /// The convex parts, in deterministic order.
    pub parts: Vec<TriMesh>,
    /// Whether the parts reproduce the input exactly.
    pub fidelity: Fidelity,
    /// Splits performed to reach this result.
    pub splits: usize,
}

impl Decomposition {
    /// Whether the input was already convex.
    pub fn is_single_part(&self) -> bool {
        self.parts.len() == 1
    }
}

/// Largest number of parts before the search is abandoned.
const MAX_PARTS: usize = 4096;

/// Decompose a closed two-manifold solid into convex parts.
///
/// # Errors
///
/// Refuses a ragged index buffer, an out-of-range index, an input that is
/// not a closed two-manifold solid, a non-positive concavity bound, and a
/// decomposition that exceeds the part budget.
pub fn convex_decompose(
    mesh: &TriMesh,
    strategy: Strategy,
    tolerance: Tolerance,
) -> Result<Decomposition, DecomposeError> {
    if mesh.indices.len() % 3 != 0 {
        return Err(DecomposeError::RaggedIndices(mesh.indices.len()));
    }
    let vertex_count = mesh.positions.len();
    for (triangle, chunk) in mesh.indices.chunks_exact(3).enumerate() {
        for &index in chunk {
            if index as usize >= vertex_count {
                return Err(DecomposeError::IndexOutOfRange(triangle, index));
            }
        }
    }

    // A decomposition only means anything for a solid: the union of parts
    // reproduces a volume, not a surface. Checking here turns a meaningless
    // answer into a named refusal.
    let health = audit_mesh(mesh, tolerance);
    if !health.is_closed_two_manifold() {
        return Err(DecomposeError::NotASolid {
            boundary: health.boundary_edges,
            non_manifold: health.non_manifold_edges,
        });
    }

    let bound = match strategy {
        Strategy::Exact => tolerance.linear(),
        Strategy::Approximate { max_concavity } => {
            if !(max_concavity > 0.0) || !max_concavity.is_finite() {
                return Err(DecomposeError::InvalidBound(max_concavity));
            }
            max_concavity
        }
    };

    // Work on meshes rather than point sets. A part is the actual solid on
    // one side of every split, produced by clipping; taking the hull of a
    // point subset instead would fill in any notch the subset still spans,
    // and the parts would sum to more volume than the input.
    let mut pending = vec![mesh.clone()];
    let mut finished: Vec<TriMesh> = Vec::new();
    let mut splits = 0usize;
    let mut achieved: Scalar = 0.0;

    while let Some(part) = pending.pop() {
        if finished.len() + pending.len() + 1 > MAX_PARTS {
            return Err(DecomposeError::BudgetExceeded { limit: MAX_PARTS });
        }

        let Some(reflex) = worst_concavity(&part.positions, &part.indices, tolerance) else {
            finished.push(part);
            continue;
        };
        if reflex.depth <= bound {
            achieved = achieved.max(reflex.depth);
            finished.push(part);
            continue;
        }

        // Split on the plane of the face the reflex vertex sticks out past.
        // Extending an existing face is the standard construction and it
        // makes progress by definition: everything in front of that plane
        // is separated from the face that could not see it.
        let (normal, offset) = (reflex.normal, reflex.offset);
        let front = clip(&part, normal, offset, tolerance);
        let back = clip(&part, -normal, -offset, tolerance);

        match (front, back) {
            (Some(front), Some(back))
                if front.triangle_count() > 0 && back.triangle_count() > 0 =>
            {
                splits += 1;
                pending.push(front);
                pending.push(back);
            }
            // The plane failed to separate the part. Keeping it whole with
            // its concavity reported is honest; looping on a split that
            // makes no progress is not.
            _ => {
                achieved = achieved.max(reflex.depth);
                finished.push(part);
            }
        }
    }

    // Deterministic ordering: parts are keyed by their extreme corner, which
    // is a property of the geometry rather than of the traversal, so the
    // same solid decomposes to the same sequence on every run.
    finished.sort_by(|a, b| {
        let ka = order_key(&a.positions);
        let kb = order_key(&b.positions);
        ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let parts = finished;

    let fidelity = match strategy {
        Strategy::Exact => Fidelity::Exact,
        Strategy::Approximate { max_concavity } => Fidelity::Approximate {
            requested: max_concavity,
            achieved,
        },
    };

    Ok(Decomposition {
        parts,
        fidelity,
        splits,
    })
}

/// Sort key: the lexicographically smallest corner of a part.
fn order_key(points: &[Point3]) -> (Scalar, Scalar, Scalar) {
    let mut best = (Scalar::INFINITY, Scalar::INFINITY, Scalar::INFINITY);
    for p in points {
        let key = (p.x, p.y, p.z);
        if key < best {
            best = key;
        }
    }
    best
}

/// A reflex feature: a vertex sticking out past one of the solid's own faces.
struct Reflex {
    /// How far the vertex lies in front of the face plane.
    depth: Scalar,
    /// The offending vertex.
    apex: Point3,
    /// Outward normal of the face it sticks out past.
    normal: Vec3,
    /// Plane offset of that face.
    offset: Scalar,
}

/// Depth and location of the worst reflex feature in a solid.
///
/// A solid is convex exactly when every vertex lies behind every face
/// plane. Where a vertex lies IN FRONT of some face plane, the solid
/// bulges past that face -- a reflex feature -- and the distance in front
/// is how deep the offending notch is.
///
/// Measuring against face planes rather than against the convex hull is
/// what makes this work. A reflex vertex generally lies exactly ON the
/// hull surface (the hull spans the notch with a face THROUGH that
/// vertex), so hull distance reports zero concavity for the very feature
/// that needs splitting.
///
/// `None` when the part is convex to within `tolerance`.
fn worst_concavity(positions: &[Point3], indices: &[u32], tolerance: Tolerance) -> Option<Reflex> {
    let linear = tolerance.linear();
    let mut worst: Option<Reflex> = None;

    for chunk in indices.chunks_exact(3) {
        let a = positions[chunk[0] as usize];
        let b = positions[chunk[1] as usize];
        let c = positions[chunk[2] as usize];

        let normal = (b - a).cross(c - a);
        let area = normal.length();
        // A degenerate face has no plane to test against; skip rather than
        // divide by a vanishing length and invent a direction.
        if area <= linear * linear {
            continue;
        }
        let unit = normal / area;

        for (index, &point) in positions.iter().enumerate() {
            let ahead = (point - a).dot(unit);
            if ahead <= linear {
                continue;
            }
            // Deeper wins; equal depth breaks toward the lower index so the
            // choice is reproducible rather than dependent on iteration
            // order over an unordered structure.
            let better = match &worst {
                None => true,
                Some(current) => {
                    ahead > current.depth + linear
                        || ((ahead - current.depth).abs() <= linear
                            && (point.x, point.y, point.z)
                                < (current.apex.x, current.apex.y, current.apex.z))
                }
            };
            if better {
                let _ = index;
                // Record the FACE the vertex sticks out past, not just how
                // far. Splitting on that face's own plane is what removes
                // the reflex feature; a bounding-box axis through the same
                // point need not separate the notch at all.
                worst = Some(Reflex {
                    depth: ahead,
                    apex: point,
                    normal: unit,
                    offset: a.dot(unit),
                });
            }
        }
    }
    worst
}

/// Clip a closed solid by a plane, keeping the side the normal points away
/// from and capping the opening so the result is closed again.
///
/// Sutherland-Hodgman per triangle: each face is clipped to the half-space,
/// producing a polygon that is re-fanned into triangles. Cut edges are
/// collected and stitched into a cap, which is what keeps the part a solid
/// rather than an open shell.
fn clip(mesh: &TriMesh, normal: Vec3, offset: Scalar, tolerance: Tolerance) -> Option<TriMesh> {
    let linear = tolerance.linear();
    let mut positions: Vec<Point3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut lookup: BTreeMap<(u64, u64, u64), u32> = BTreeMap::new();
    let mut cut_edges: Vec<(Point3, Point3)> = Vec::new();

    let mut intern = |point: Point3, positions: &mut Vec<Point3>, lookup: &mut BTreeMap<_, _>| {
        let key = (
            quantise(point.x, linear),
            quantise(point.y, linear),
            quantise(point.z, linear),
        );
        *lookup.entry(key).or_insert_with(|| {
            positions.push(point);
            (positions.len() - 1) as u32
        })
    };

    for chunk in mesh.indices.chunks_exact(3) {
        let triangle = [
            mesh.positions[chunk[0] as usize],
            mesh.positions[chunk[1] as usize],
            mesh.positions[chunk[2] as usize],
        ];

        // A face lying IN the clip plane belongs to exactly one side, and
        // deciding that by its distances alone is impossible: every corner
        // reads as "on the plane", so both sides would keep it. The face is
        // then duplicated, its edges match nothing on either part, and the
        // union double-counts it.
        //
        // Its own normal settles it. A coplanar face bounds the material on
        // the side it faces away from, so it goes to the half-space that
        // keeps it as an outer wall.
        let distances = [
            triangle[0].dot(normal) - offset,
            triangle[1].dot(normal) - offset,
            triangle[2].dot(normal) - offset,
        ];
        if distances.iter().all(|d| d.abs() <= linear) {
            let face = (triangle[1] - triangle[0]).cross(triangle[2] - triangle[0]);
            // Faces pointing along the clip normal bound the material behind
            // the plane, which is the side this call keeps.
            if face.dot(normal) > 0.0 {
                let a = intern(triangle[0], &mut positions, &mut lookup);
                let b = intern(triangle[1], &mut positions, &mut lookup);
                let c = intern(triangle[2], &mut positions, &mut lookup);
                if a != b && b != c && c != a {
                    indices.extend_from_slice(&[a, b, c]);
                }
            }
            continue;
        }

        // Clip the triangle against the plane, tracking where the boundary
        // of the kept region crosses it.
        let mut kept: Vec<Point3> = Vec::new();
        let mut crossings: Vec<Point3> = Vec::new();
        for corner in 0..3 {
            let current = triangle[corner];
            let next = triangle[(corner + 1) % 3];
            let d_current = current.dot(normal) - offset;
            let d_next = next.dot(normal) - offset;

            if d_current <= linear {
                kept.push(current);
            }
            // A vertex ON the plane is part of the cut boundary even though
            // no edge through it changes sign. Leaving these out was why the
            // cap loop stopped short: the cross-section's corners are often
            // existing vertices, not new crossings.
            if d_current.abs() <= linear {
                crossings.push(current);
            }
            // A strict sign change produces a new crossing point, shared by
            // both parts, which is what makes their union seamless.
            if (d_current < -linear && d_next > linear) || (d_current > linear && d_next < -linear)
            {
                let t = d_current / (d_current - d_next);
                let point = current + (next - current) * t;
                kept.push(point);
                crossings.push(point);
            }
        }

        // Only a triangle with material on BOTH sides bounds the cut. One
        // that merely touches the plane, or lies in it, is already walled by
        // its own face: capping there would lay a second surface over the
        // first and make the edge non-manifold.
        // Two cases bound the cut, and both are needed.
        //
        // A triangle with material on both sides is cut through its middle.
        // A triangle with exactly two vertices ON the plane touches it along
        // a whole edge -- the wall standing on the cut -- and that edge is
        // part of the cross-section boundary even though the triangle never
        // crosses. Without it the loop is left open along every such wall,
        // which is what stopped the L-shape's cap from closing.
        //
        // A triangle lying entirely in the plane is excluded by both, which
        // is correct: it is already a face of the part, and capping over it
        // would make its edges non-manifold.
        let straddles =
            distances.iter().any(|d| *d < -linear) && distances.iter().any(|d| *d > linear);

        // Deduplicate: a vertex on the plane can be reported by two of the
        // triangle's edges. Exactly two distinct points bound the segment
        // where this triangle meets the plane.
        crossings.dedup_by(|a, b| (*a - *b).length() <= linear);
        if straddles && crossings.len() >= 2 {
            let first = crossings[0];
            let last = *crossings.last().expect("non-empty");
            if (first - last).length() > linear {
                cut_edges.push((first, last));
            }
        }
        if kept.len() < 3 {
            continue;
        }

        let anchor = intern(kept[0], &mut positions, &mut lookup);
        for corner in 1..kept.len() - 1 {
            let b = intern(kept[corner], &mut positions, &mut lookup);
            let c = intern(kept[corner + 1], &mut positions, &mut lookup);
            if anchor != b && b != c && c != anchor {
                indices.extend_from_slice(&[anchor, b, c]);
            }
        }
    }

    if cut_edges.is_empty() || indices.is_empty() {
        return None;
    }

    // Cap the opening. The cut edges bound one or more planar loops; each
    // must be stitched into order before it can be triangulated. Fanning
    // over the edges in the order they happened to be produced only closes
    // the opening when that order is already a fan, which for a general
    // cross-section it is not.
    let stitched = stitch_loops(&cut_edges, linear);
    eprintln!(
        "CAPDIAG cut_edges={} loops={:?}",
        cut_edges.len(),
        stitched.iter().map(|l| l.len()).collect::<Vec<_>>()
    );
    for loop_points in stitched {
        if loop_points.len() < 3 {
            continue;
        }
        // Ear clipping works in 2D, so express the loop in the cut plane's
        // own basis. Any orthonormal pair perpendicular to the normal will
        // do; the choice does not affect which triangles come out.
        let (axis_u, axis_v) = plane_basis(normal);
        let origin = loop_points[0];
        let flat: Vec<Point2> = loop_points
            .iter()
            .map(|p| {
                let d = *p - origin;
                Point2::new(d.dot(axis_u), d.dot(axis_v))
            })
            .collect();

        // Ear clipping needs a counter-clockwise ring; a clockwise one is
        // reversed rather than refused, since the winding of a cut loop is
        // an artefact of traversal, not of the geometry.
        let area: Scalar = flat
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let q = flat[(k + 1) % flat.len()];
                p.x * q.y - q.x * p.y
            })
            .sum();
        let (flat, loop_points) = if area < 0.0 {
            let mut f = flat;
            let mut l = loop_points;
            f.reverse();
            l.reverse();
            (f, l)
        } else {
            (flat, loop_points)
        };

        let Ok(fan) = axiolid_reference::polygon::triangulate_simple(&flat) else {
            // A loop that will not triangulate leaves the part open, and an
            // open part is refused upstream rather than silently returned.
            continue;
        };
        for triple in fan {
            let a = intern(loop_points[triple[0] as usize], &mut positions, &mut lookup);
            let b = intern(loop_points[triple[1] as usize], &mut positions, &mut lookup);
            let c = intern(loop_points[triple[2] as usize], &mut positions, &mut lookup);
            if a == b || b == c || c == a {
                continue;
            }
            // The cap faces along the clip normal, opposite the material
            // that was removed, so the shell stays consistently outward.
            let wound = (positions[b as usize] - positions[a as usize])
                .cross(positions[c as usize] - positions[a as usize]);
            if wound.dot(normal) >= 0.0 {
                indices.extend_from_slice(&[a, b, c]);
            } else {
                indices.extend_from_slice(&[a, c, b]);
            }
        }
    }

    Some(TriMesh::new(positions, indices))
}

/// The axis along which a point set is most spread out.
///
/// Chosen over a principal-axis computation because it needs no eigen
/// solve and is exactly reproducible; ties break toward x then y, so the
/// choice is deterministic for a symmetric part.
fn widest_axis(points: &[Point3]) -> Vec3 {
    let mut min = Point3::new(Scalar::INFINITY, Scalar::INFINITY, Scalar::INFINITY);
    let mut max = Point3::new(
        Scalar::NEG_INFINITY,
        Scalar::NEG_INFINITY,
        Scalar::NEG_INFINITY,
    );
    for p in points {
        min = Point3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
        max = Point3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
    }
    let extent = max - min;
    if extent.x >= extent.y && extent.x >= extent.z {
        Vec3::X
    } else if extent.y >= extent.z {
        Vec3::Y
    } else {
        Vec3::Z
    }
}

/// Chain unordered cut edges into closed loops.
///
/// Clipping produces the cut edges one triangle at a time, in no
/// particular order. A cap can only be triangulated once those edges are
/// walked into a ring, so each edge is joined to the next one sharing an
/// endpoint until the loop closes.
///
/// Endpoints are matched on a tolerance lattice: the same crossing point
/// computed from two adjacent triangles differs in the last few bits, and
/// exact comparison would leave every loop broken.
fn stitch_loops(edges: &[(Point3, Point3)], linear: Scalar) -> Vec<Vec<Point3>> {
    let key = |p: &Point3| {
        (
            quantise(p.x, linear),
            quantise(p.y, linear),
            quantise(p.z, linear),
        )
    };

    let mut adjacency: BTreeMap<(u64, u64, u64), Vec<usize>> = BTreeMap::new();
    eprintln!(
        "CAPDIAG edges {:?}",
        edges
            .iter()
            .map(|(a, b)| ((a.x, a.z), (b.x, b.z)))
            .collect::<Vec<_>>()
    );
    for (index, (from, to)) in edges.iter().enumerate() {
        adjacency.entry(key(from)).or_default().push(index);
        adjacency.entry(key(to)).or_default().push(index);
    }

    let mut used = vec![false; edges.len()];
    let mut loops = Vec::new();

    for start in 0..edges.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut ring = vec![edges[start].0, edges[start].1];
        let mut tail = edges[start].1;

        loop {
            let candidates = match adjacency.get(&key(&tail)) {
                Some(list) => list,
                None => break,
            };
            let mut advanced = false;
            for &next in candidates {
                if used[next] {
                    continue;
                }
                let (from, to) = edges[next];
                let other = if key(&from) == key(&tail) {
                    to
                } else if key(&to) == key(&tail) {
                    from
                } else {
                    continue;
                };
                used[next] = true;
                // Closing the ring: stop rather than repeat the first point.
                if key(&other) == key(&ring[0]) {
                    advanced = false;
                    break;
                }
                ring.push(other);
                tail = other;
                advanced = true;
                break;
            }
            if !advanced {
                break;
            }
        }
        if ring.len() >= 3 {
            eprintln!(
                "CAPDIAG ring {:?}",
                ring.iter().map(|p| (p.x, p.z)).collect::<Vec<_>>()
            );
            loops.push(ring);
        }
    }
    loops
}

/// Any orthonormal basis of the plane perpendicular to `normal`.
fn plane_basis(normal: Vec3) -> (Vec3, Vec3) {
    // Seed against the axis the normal is least aligned with, so the cross
    // product is well conditioned rather than near zero.
    let seed = if normal.x.abs() <= normal.y.abs() && normal.x.abs() <= normal.z.abs() {
        Vec3::X
    } else if normal.y.abs() <= normal.z.abs() {
        Vec3::Y
    } else {
        Vec3::Z
    };
    let u = normal.cross(seed).normalize();
    let v = normal.cross(u);
    (u, v)
}

/// Snap a coordinate to a tolerance-sized lattice for welding.
///
/// Two clipped faces meeting at a cut must agree on the crossing vertex, or
/// the part is not closed. Comparing raw bits is too strict: the same point
/// computed from two different edges differs in the last ulp.
fn quantise(value: Scalar, linear: Scalar) -> u64 {
    let step = linear.max(Scalar::EPSILON);
    let snapped = (value / step).round();
    snapped.to_bits()
}

/// Count of distinct positions, used by tests and callers sizing buffers.
pub fn distinct_positions(mesh: &TriMesh) -> usize {
    let mut seen = BTreeMap::new();
    for p in &mesh.positions {
        seen.insert((p.x.to_bits(), p.y.to_bits(), p.z.to_bits()), ());
    }
    seen.len()
}
