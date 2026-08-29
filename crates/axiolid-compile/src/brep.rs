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
) -> GeomResult<TriMesh> {
    let solid = brep
        .solids()
        .first()
        .ok_or_else(|| GeomError::InvalidInput("brep has no solid".to_string()))?;
    let shell = brep
        .shells()
        .get(solid.outer.index())
        .ok_or_else(|| GeomError::InvalidInput("outer shell missing".to_string()))?;

    let mut mesh = TriMesh::default();
    let mut welded: std::collections::HashMap<axiolid_topology::VertexId, u32> =
        std::collections::HashMap::new();
    for &(face_id, shell_sense) in &shell.faces {
        let face = brep
            .faces()
            .get(face_id.index())
            .ok_or_else(|| GeomError::InvalidInput("face missing".to_string()))?;
        let flip =
            (shell_sense == Orientation::Reversed) ^ (face.orientation == Orientation::Reversed);
        append_face(&mut mesh, brep, face, graph, flip, &mut welded)?;
    }
    Ok(mesh)
}

/// Triangulate one face and append it to the mesh.
fn append_face(
    mesh: &mut TriMesh,
    brep: &BRep<NodeId>,
    face: &axiolid_topology::Face<NodeId>,
    graph: &axiolid_model::GeometryGraph,
    flip: bool,
    welded: &mut std::collections::HashMap<axiolid_topology::VertexId, u32>,
) -> GeomResult<()> {
    // A face carrying a curved support surface cannot be tessellated by
    // projecting its boundary onto a plane: the interior curves away from
    // that plane and the error is invisible in the output. Refuse instead.
    // `axiolid-compile/AGENTS.md`: a missing wall is cheap, a wrong wall
    // corrupts every downstream quantity.
    if let Some(surface) = face_surface(graph, face)? {
        if !surface_is_planar(surface) {
            return Err(GeomError::Unsupported {
                backend: crate::BACKEND_ID,
                operation: axiolid_kernel::Operation::Tessellation,
            });
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
