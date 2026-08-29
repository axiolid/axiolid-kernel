//! Faceted B-rep tessellation.
//!
//! A brep face is a planar polygon in an arbitrary plane, possibly
//! concave or holed. A triangle fan is wrong for both, so each face is
//! projected to its own plane, triangulated with the same earcut path
//! profiles use, and lifted back. Shared vertices stay shared: the loop
//! indices already reference interned topology vertices.

use axiolid_core::{Scalar, Vec3};
use axiolid_kernel::{GeomError, GeomResult};
use axiolid_mesh::TriMesh;
use axiolid_model::NodeId;
use axiolid_topology::{BRep, Orientation};

/// Tessellate one faceted B-rep into a triangle mesh.
///
/// Only the outer shell contributes surface; void shells are interior
/// boundaries whose removal is a boolean, not a tessellation, so emitting
/// them here would produce a mesh with stray inside-out geometry.
pub fn tessellate(
    brep: &BRep<NodeId>,
    graph: &axiolid_model::GeometryGraph,
    tolerance: axiolid_core::Tolerance,
) -> GeomResult<TriMesh> {
    // Structure before geometry. A dangling handle or an open loop
    // produces a mesh that looks plausible and is wrong, so the
    // topology is audited before a single triangle is emitted.
    let health = axiolid_topology::audit_brep(brep);
    if !health.is_tessellable() {
        return Err(GeomError::InvalidInput(format!(
            "brep topology is not tessellable: {health:?}"
        )));
    }

    let solid = brep
        .solids()
        .first()
        .ok_or_else(|| GeomError::InvalidInput("brep has no solid".to_string()))?;
    let shell = brep
        .shells()
        .get(solid.outer.index())
        .ok_or_else(|| GeomError::InvalidInput("outer shell missing".to_string()))?;

    let mut mesh = TriMesh::default();
    let ctx = FaceContext {
        brep,
        graph,
        tolerance,
    };
    let mut edge_cache: EdgeSamples = EdgeSamples::new();
    let mut welded: std::collections::HashMap<axiolid_topology::VertexId, u32> =
        std::collections::HashMap::new();
    for &(face_id, shell_sense) in &shell.faces {
        let face = brep
            .faces()
            .get(face_id.index())
            .ok_or_else(|| GeomError::InvalidInput("face missing".to_string()))?;
        let flip =
            (shell_sense == Orientation::Reversed) ^ (face.orientation == Orientation::Reversed);
        append_face(&mut mesh, &ctx, face, flip, &mut welded, &mut edge_cache)?;
    }
    Ok(mesh)
}

/// Triangulate one face and append it to the mesh.
/// Everything a face needs that does not vary within one shell.
struct FaceContext<'a> {
    brep: &'a BRep<NodeId>,
    graph: &'a axiolid_model::GeometryGraph,
    tolerance: axiolid_core::Tolerance,
}

fn append_face(
    mesh: &mut TriMesh,
    ctx: &FaceContext<'_>,
    face: &axiolid_topology::Face<NodeId>,
    flip: bool,
    welded: &mut std::collections::HashMap<axiolid_topology::VertexId, u32>,
    cache: &mut EdgeSamples,
) -> GeomResult<()> {
    let (brep, graph) = (ctx.brep, ctx.graph);
    // A face carrying a curved support surface cannot be tessellated by
    // projecting its boundary onto a plane: the interior curves away from
    // that plane and the error is invisible in the output. Refuse instead.
    // `axiolid-compile/AGENTS.md`: a missing wall is cheap, a wrong wall
    // corrupts every downstream quantity.
    // A curved support cannot be tessellated by projecting the boundary onto
    // a plane: the interior curves away from it and the error is invisible in
    // the output. Sample the surface itself when the face states its boundary
    // in surface parameters, and refuse when it does not.
    if let Some(surface) = face_surface(graph, face)? {
        if !surface_is_planar(surface) {
            return append_curved_face(mesh, ctx, face, surface, cache, flip);
        }
    }
    let mut rings: Vec<Vec<(axiolid_topology::VertexId, Vec3)>> = Vec::new();
    let mut outer_index = None;
    for bound in &face.bounds {
        let points = loop_points(brep, bound)?;
        if points.len() < 3 {
            continue;
        }
        if bound.outer && outer_index.is_none() {
            outer_index = Some(rings.len());
        }
        rings.push(points);
    }
    if rings.is_empty() {
        return Ok(());
    }
    // A face whose bounds are all tagged inner still has an outer boundary;
    // fall back to the first ring rather than dropping the facet.
    let outer_index = outer_index.unwrap_or(0);
    rings.swap(0, outer_index);

    let outer_positions: Vec<Vec3> = rings[0].iter().map(|(_, p)| *p).collect();
    let normal = newell_normal(&outer_positions);
    let Some((u, v)) = plane_axes(normal) else {
        // A zero-area ring defines no plane; skip rather than emit garbage.
        return Ok(());
    };
    let origin = rings[0][0].1;

    let mut flat: Vec<[Scalar; 2]> = Vec::new();
    let mut positions: Vec<(axiolid_topology::VertexId, Vec3)> = Vec::new();
    let mut hole_starts: Vec<usize> = Vec::new();
    for (index, ring) in rings.iter().enumerate() {
        if index > 0 {
            hole_starts.push(flat.len());
        }
        for &(vertex, point) in ring {
            let d = point - origin;
            flat.push([d.dot(u), d.dot(v)]);
            positions.push((vertex, point));
        }
    }

    let mut earcutter = earcut::Earcut::new();
    let mut indices: Vec<usize> = Vec::new();
    earcutter.earcut(flat.iter().copied(), &hole_starts, &mut indices);
    if indices.is_empty() || indices.len() % 3 != 0 {
        return Err(GeomError::Degenerate(format!(
            "face triangulation produced {} indices for {} vertices",
            indices.len(),
            flat.len()
        )));
    }

    // Weld by topological vertex. Adjacent facets already share interned
    // vertices upstream; emitting per-face copies would leave every edge
    // unshared, so the mesh would look correct yet fail a manifold check.
    let mut local: Vec<u32> = Vec::with_capacity(positions.len());
    for &(vertex, point) in &positions {
        let index = *welded.entry(vertex).or_insert_with(|| {
            let next = mesh.positions.len() as u32;
            mesh.positions.push(point);
            next
        });
        local.push(index);
    }

    for triangle in indices.chunks_exact(3) {
        let (a, b, c) = (local[triangle[0]], local[triangle[1]], local[triangle[2]]);
        if flip {
            mesh.indices.extend([a, c, b]);
        } else {
            mesh.indices.extend([a, b, c]);
        }
    }
    Ok(())
}

/// Walk a bound loop and collect its vertex positions in order.
fn loop_points(
    brep: &BRep<NodeId>,
    bound: &axiolid_topology::FaceBound,
) -> GeomResult<Vec<(axiolid_topology::VertexId, Vec3)>> {
    let wire = brep
        .loops()
        .get(bound.loop_id.index())
        .ok_or_else(|| GeomError::InvalidInput("loop missing".to_string()))?;
    let mut points: Vec<(axiolid_topology::VertexId, Vec3)> = Vec::with_capacity(wire.edges.len());
    for use_ in &wire.edges {
        let edge = brep
            .edges()
            .get(use_.edge.index())
            .ok_or_else(|| GeomError::InvalidInput("edge missing".to_string()))?;
        // Each edge contributes its start under traversal orientation; the
        // loop is closed, so the final end repeats the first start.
        let vertex = if use_.orientation == Orientation::Forward {
            edge.start
        } else {
            edge.end
        };
        let position = brep
            .vertices()
            .get(vertex.index())
            .ok_or_else(|| GeomError::InvalidInput("vertex missing".to_string()))?
            .position;
        points.push((vertex, position));
    }
    if bound.orientation == Orientation::Reversed {
        points.reverse();
    }
    Ok(points)
}

/// Newell normal: correct for concave and non-planar-ish polygons alike.
///
/// A cross product of the first two edges fails when they are collinear,
/// which is common at the start of an exported ring.
fn newell_normal(ring: &[Vec3]) -> Vec3 {
    let mut normal = Vec3::ZERO;
    for index in 0..ring.len() {
        let current = ring[index];
        let next = ring[(index + 1) % ring.len()];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    normal
}

/// Orthonormal in-plane axes for a normal, or None when it is degenerate.
fn plane_axes(normal: Vec3) -> Option<(Vec3, Vec3)> {
    let length = normal.length();
    if length <= f64::EPSILON {
        return None;
    }
    let n = normal / length;
    // Pick the axis least aligned with n so the cross product stays stable.
    let helper = if n.x.abs() <= n.y.abs() && n.x.abs() <= n.z.abs() {
        Vec3::X
    } else if n.y.abs() <= n.z.abs() {
        Vec3::Y
    } else {
        Vec3::Z
    };
    let u = n.cross(helper).normalize();
    let v = n.cross(u);
    Some((u, v))
}

/// Whether an exact support surface is planar.
///
/// A planar support adds nothing the boundary polygon does not already say,
/// so those faces stay on the fast path. Every other surface family
/// curves away from the boundary plane, and projecting it there is a
/// silent modelling error.
fn surface_is_planar(surface: &axiolid_surface::Surface) -> bool {
    matches!(surface, axiolid_surface::Surface::Plane(_))
}

/// Resolve a face's support surface from the graph, if it declares one.
///
/// A face may legitimately omit its surface: a planar polygon's boundary
/// already determines its plane. A declared handle that does not resolve to
/// a Surface node is a graph error, not an absent surface.
fn face_surface<'g>(
    graph: &'g axiolid_model::GeometryGraph,
    face: &axiolid_topology::Face<NodeId>,
) -> GeomResult<Option<&'g axiolid_surface::Surface>> {
    let Some(id) = face.surface else {
        return Ok(None);
    };
    match graph.get(id) {
        Some(axiolid_model::GeometryNode::Surface(s)) => Ok(Some(s)),
        Some(_) => Err(GeomError::InvalidInput(format!(
            "face support {id:?} is not a Surface node"
        ))),
        None => Err(GeomError::InvalidInput(format!(
            "face support {id:?} does not belong to this graph"
        ))),
    }
}

/// Tessellate a face whose support surface is curved.
///
/// The face's boundary must be stated in the surface's parameter domain:
/// every edge use needs a pcurve. Without them the trim is unknown, and
/// guessing it by inverting the surface is not generally solvable. A
/// missing pcurve is therefore refused, not approximated.
///
/// The parameter-space boundary gives a `(u, v)` polygon. Its bounding box
/// is the patch actually sampled, so a half-cylinder costs half a cylinder
/// rather than a full revolution clipped afterwards.
// Shared edge samples, keyed canonically in the edge's start->end direction.
type EdgeSamples = std::collections::HashMap<axiolid_topology::EdgeId, (Vec<u32>, Orientation)>;

/// Sample a 2D trim curve at `n` uniform parameters.
///
/// Used when an edge is already interned: the second face must produce the
/// same NUMBER of boundary points as the first, so its (u, v) values pair
/// one-to-one with the shared 3D indices.
fn pcurve_points_n(
    graph: &axiolid_model::GeometryGraph,
    id: NodeId,
    n: usize,
) -> GeomResult<Vec<axiolid_core::Point2>> {
    let Some(axiolid_model::GeometryNode::Curve2(curve)) = graph.get(id) else {
        return Err(GeomError::InvalidInput(
            "edge pcurve must reference a Curve2 node".to_string(),
        ));
    };
    let domain = axiolid_scalar::curve::domain2(curve);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = domain.start + (domain.end - domain.start) * (i as Scalar) / (n as Scalar);
        out.push(axiolid_scalar::curve::evaluate2(curve, t)?);
    }
    Ok(out)
}

/// Boundary of a curved face in parameter space, paired with the shared 3D
/// vertex index for each point.
///
/// The pairing is what makes a seam watertight: (u, v) drives triangulation,
/// and the index says which already-created vertex to emit.
struct CurvedBoundary {
    uv: Vec<axiolid_core::Point2>,
    shared: Vec<u32>,
    hole_starts: Vec<usize>,
}

/// Collect a curved face's boundary, sampling each edge exactly once across
/// the whole shell.
fn curved_boundary(
    mesh: &mut TriMesh,
    ctx: &FaceContext<'_>,
    face: &axiolid_topology::Face<NodeId>,
    surface: &axiolid_surface::Surface,
    cache: &mut EdgeSamples,
) -> GeomResult<CurvedBoundary> {
    let (brep, graph, tolerance) = (ctx.brep, ctx.graph, ctx.tolerance);
    let mut out = CurvedBoundary {
        uv: Vec::new(),
        shared: Vec::new(),
        hole_starts: Vec::new(),
    };
    for (index, bound) in face.bounds.iter().enumerate() {
        if index > 0 {
            out.hole_starts.push(out.uv.len());
        }
        let wire = brep
            .loops()
            .get(bound.loop_id.index())
            .ok_or_else(|| GeomError::InvalidInput("loop missing".to_string()))?;
        for use_ in &wire.edges {
            let Some(pcurve) = use_.pcurve else {
                return Err(GeomError::Unsupported {
                    backend: crate::BACKEND_ID,
                    operation: axiolid_kernel::Operation::Tessellation,
                });
            };
            let cached = cache.get(&use_.edge).map(|(v, _)| v.len());
            // First face to reach this edge chooses the sample count from
            // its own chord budget. A later face must match that count so
            // its (u, v) points pair one-to-one with the shared vertices.
            let params = match cached {
                Some(n) => pcurve_points_n(graph, pcurve, n)?,
                None => {
                    let mut p = pcurve_points(graph, pcurve, tolerance)?;
                    p.pop();
                    p
                }
            };
            // Evaluate the trim on the surface: these are the seam's
            // 3D points, and they are interned under the edge.
            let points: Vec<Vec3> = params
                .iter()
                .map(|p| axiolid_scalar::surface::evaluate(surface, p.x, p.y))
                .collect::<GeomResult<_>>()?;
            let shared = edge_samples(mesh, cache, use_.edge, use_.orientation, &points);
            out.uv.extend(params);
            out.shared.extend(shared);
        }
    }
    Ok(out)
}

/// Intern one edge's 3D samples, or return the existing ones.
///
/// Interning is keyed by EDGE, not by face. Whichever face reaches a seam
/// first creates the vertices; every later face reuses the identical
/// indices, so the seam is shared rather than merely coincident.
fn edge_samples(
    mesh: &mut TriMesh,
    cache: &mut EdgeSamples,
    edge: axiolid_topology::EdgeId,
    sense: Orientation,
    points: &[Vec3],
) -> Vec<u32> {
    if let Some((existing, stored)) = cache.get(&edge) {
        let mut out = existing.clone();
        // Walk the shared vertices in this use's direction.
        if *stored != sense {
            out.reverse();
        }
        return out;
    }
    let indices: Vec<u32> = points
        .iter()
        .map(|p| {
            let next = mesh.positions.len() as u32;
            mesh.positions.push(*p);
            next
        })
        .collect();
    cache.insert(edge, (indices.clone(), sense));
    indices
}

/// Tessellate a face whose support surface is curved.
///
/// The boundary comes from shared edge samples, so a seam between two curved
/// faces uses one set of vertices. Interior detail still comes from the
/// surface: the boundary alone would flatten the patch.
fn append_curved_face(
    mesh: &mut TriMesh,
    ctx: &FaceContext<'_>,
    face: &axiolid_topology::Face<NodeId>,
    surface: &axiolid_surface::Surface,
    cache: &mut EdgeSamples,
    flip: bool,
) -> GeomResult<()> {
    let boundary = curved_boundary(mesh, ctx, face, surface, cache)?;
    if boundary.uv.len() < 3 {
        return Ok(());
    }
    let flat: Vec<[Scalar; 2]> = boundary.uv.iter().map(|p| [p.x, p.y]).collect();
    let mut earcutter = earcut::Earcut::new();
    let mut indices: Vec<usize> = Vec::new();
    earcutter.earcut(flat.iter().copied(), &boundary.hole_starts, &mut indices);
    if indices.is_empty() || indices.len() % 3 != 0 {
        return Err(GeomError::Degenerate(format!(
            "curved face trim triangulation produced {} indices for {} points",
            indices.len(),
            flat.len()
        )));
    }
    for t in indices.chunks_exact(3) {
        let (a, b, c) = (
            boundary.shared[t[0]],
            boundary.shared[t[1]],
            boundary.shared[t[2]],
        );
        if flip {
            mesh.indices.extend([a, c, b]);
        } else {
            mesh.indices.extend([a, b, c]);
        }
    }
    Ok(())
}

/// Flatten a 2D trim curve into parameter-space points.
fn pcurve_points(
    graph: &axiolid_model::GeometryGraph,
    id: NodeId,
    tolerance: axiolid_core::Tolerance,
) -> GeomResult<Vec<axiolid_core::Point2>> {
    let node = graph
        .get(id)
        .ok_or_else(|| GeomError::InvalidInput(format!("pcurve {id:?} is not in this graph")))?;
    let axiolid_model::GeometryNode::Curve2(curve) = node else {
        return Err(GeomError::InvalidInput(format!(
            "pcurve {id:?} must be a Curve2 node"
        )));
    };
    let domain = axiolid_scalar::curve::domain2(curve);
    axiolid_scalar::curve::flatten2(curve, domain, tolerance.linear(), 20)
}
