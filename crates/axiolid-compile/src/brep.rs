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
            return append_curved_face(mesh, ctx, welded, face, surface, cache, flip);
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

/// Recognise a boundary as a rectangular patch, or decline.
///
/// Declining is not a failure: a trimmed face with a hole, a slanted trim
/// or an irregular sample layout is genuinely not a grid, and earcut
/// remains the right tool for it. This only claims the cases it can prove.
fn recognise_grid(boundary: &CurvedBoundary, tolerance: Scalar) -> Option<GridPatch> {
    // A hole means the patch is not simply a rectangle.
    if !boundary.hole_starts.is_empty() || boundary.uv.len() < 4 {
        return None;
    }
    let (mut u_min, mut u_max) = (Scalar::INFINITY, Scalar::NEG_INFINITY);
    let (mut v_min, mut v_max) = (Scalar::INFINITY, Scalar::NEG_INFINITY);
    for p in &boundary.uv {
        u_min = u_min.min(p.x);
        u_max = u_max.max(p.x);
        v_min = v_min.min(p.y);
        v_max = v_max.max(p.y);
    }
    let (u_span, v_span) = (u_max - u_min, v_max - v_min);
    if !(u_span > 0.0 && v_span > 0.0) {
        return None;
    }
    // Every boundary point must lie ON the rectangle's border, and the
    // spacing must be uniform. Counting distinct coordinates recovers the
    // cell counts without assuming which side the walk started on.
    let on_edge = |value: Scalar, lo: Scalar, hi: Scalar, span: Scalar| {
        (value - lo).abs() <= span * 1e-9 || (value - hi).abs() <= span * 1e-9
    };
    let mut us: Vec<Scalar> = Vec::new();
    let mut vs: Vec<Scalar> = Vec::new();
    for p in &boundary.uv {
        let u_on = on_edge(p.x, u_min, u_max, u_span);
        let v_on = on_edge(p.y, v_min, v_max, v_span);
        if !u_on && !v_on {
            return None;
        }
        if v_on {
            us.push(p.x);
        }
        if u_on {
            vs.push(p.y);
        }
    }
    let nu = distinct_steps(&mut us, u_min, u_span)?;
    let nv = distinct_steps(&mut vs, v_min, v_span)?;
    if nu == 0 || nv == 0 {
        return None;
    }
    let _ = tolerance;
    Some(GridPatch {
        nu,
        nv,
        u_start: u_min,
        v_start: v_min,
        du: u_span / nu as Scalar,
        dv: v_span / nv as Scalar,
    })
}

/// Count uniform cells spanned by a set of coordinates, or decline.
///
/// Non-uniform spacing means the sides were sampled at different rates and
/// a grid would not line up with the boundary, so the caller must fall
/// back rather than weld mismatched vertices.
fn distinct_steps(values: &mut Vec<Scalar>, start: Scalar, span: Scalar) -> Option<usize> {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    values.dedup_by(|a, b| (*a - *b).abs() <= span * 1e-9);
    // At least two distinct values survive dedup whenever the span is
    // positive, which recognise_grid has already established, so cells
    // is at least one and the division below is safe.
    let cells = values.len().checked_sub(1)?;
    let step = span / cells as Scalar;
    for (index, value) in values.iter().enumerate() {
        let want = start + step * index as Scalar;
        if (value - want).abs() > span * 1e-6 {
            return None;
        }
    }
    Some(cells)
}

/// Mesh a rectangular patch as a structured grid.
///
/// Boundary vertices are reused, never duplicated: the caller already
/// interned them under their topological identity, so a neighbouring face
/// shares them and the shell stays closed. Interior vertices are evaluated
/// on the surface, which is what stops the patch from being flattened to
/// its boundary.
///
/// A periodic patch needs no special wrap. Its seam is a real edge of the
/// face, traversed twice by the loop, so the boundary walk resolves BOTH
/// traversals to one set of shared vertices and supplies them for every
/// row of the grid. The u = 0 and u = period columns therefore hold the
/// same vertex ids already, and the surface closes without index
/// arithmetic. This was measured, not assumed: forcing a wrap off and
/// counting freshly evaluated vertices that coincide with an existing
/// position yields zero.
fn grid_vertices(
    mesh: &mut TriMesh,
    boundary: &CurvedBoundary,
    patch: &GridPatch,
    surface: &axiolid_surface::Surface,
) -> GeomResult<Vec<u32>> {
    let columns = patch.nu + 1;
    let rows = patch.nv + 1;
    // Index boundary points by their grid cell so existing vertices win.
    let mut lookup: std::collections::HashMap<(usize, usize), u32> =
        std::collections::HashMap::with_capacity(boundary.uv.len());
    for (position, p) in boundary.uv.iter().enumerate() {
        let iu = ((p.x - patch.u_start) / patch.du).round();
        let iv = ((p.y - patch.v_start) / patch.dv).round();
        if iu < 0.0 || iv < 0.0 {
            continue;
        }
        let (iu, iv) = (iu as usize, iv as usize);
        if iu <= patch.nu && iv <= patch.nv {
            lookup.insert((iu, iv), boundary.shared[position]);
        }
    }
    let mut grid = Vec::with_capacity(columns * rows);
    for iv in 0..rows {
        for iu in 0..columns {
            if let Some(&existing) = lookup.get(&(iu, iv)) {
                grid.push(existing);
                continue;
            }
            let u = patch.u_start + patch.du * iu as Scalar;
            let v = patch.v_start + patch.dv * iv as Scalar;
            let point = axiolid_scalar::surface::evaluate(surface, u, v)?;
            let index = mesh.positions.len() as u32;
            mesh.positions.push(point);
            grid.push(index);
        }
    }
    Ok(grid)
}

/// The parameter-space boundary gives a `(u, v)` polygon. Its bounding box
/// is the patch actually sampled, so a half-cylinder costs half a cylinder
/// rather than a full revolution clipped afterwards.
// Shared edge samples, keyed canonically in the edge's start->end direction.
type EdgeSamples = std::collections::HashMap<axiolid_topology::EdgeId, (Vec<u32>, Orientation)>;

/// Samples an edge needs so its 3D image meets the chord budget.
///
/// The trim is flattened in PARAMETER space, but tolerance is a claim about
/// 3D. A cylinder rim is the straight pcurve v = 0,
/// u in [0, TAU]: two points in parameter space, a full circle in space.
/// Subdivide until the mapped midpoint is within tolerance of the chord.
fn edge_sample_count(
    curve: &axiolid_curve::Curve2,
    surface: &axiolid_surface::Surface,
    tolerance: Scalar,
) -> GeomResult<usize> {
    let domain = axiolid_scalar::curve::domain2(curve);
    let at = |t: Scalar| -> GeomResult<Vec3> {
        let p = axiolid_scalar::curve::evaluate2(curve, t)?;
        axiolid_scalar::surface::evaluate(surface, p.x, p.y)
    };
    // Start at one segment, not two. A trim whose 3D image is straight --
    // the seam of a cylinder is the obvious case, u constant and v running
    // the height -- is represented exactly by its endpoints. Starting at
    // two forced an interior sample onto such an edge, and because the
    // surrounding rims are sampled independently at a much higher count,
    // that sample had no counterpart on the adjacent column: a hanging
    // node, which is a T-junction and leaves the mesh cracked at the seam.
    //
    // Powers of two keep the count stable: a face arriving later computes
    // the same value from the same inputs.
    let mut n = 1usize;
    while n < 4096 {
        let mut worst: Scalar = 0.0;
        for i in 0..n {
            let t0 = domain.start + (domain.end - domain.start) * (i as Scalar) / (n as Scalar);
            let t1 =
                domain.start + (domain.end - domain.start) * ((i + 1) as Scalar) / (n as Scalar);
            let (a, b) = (at(t0)?, at(t1)?);
            let m = at(0.5 * (t0 + t1))?;
            worst = worst.max((m - 0.5 * (a + b)).length());
        }
        if worst <= tolerance {
            break;
        }
        n *= 2;
    }
    Ok(n)
}

/// Sample a trim at exactly `n` points, excluding the closing endpoint.
///
/// The next edge in the loop contributes the shared junction, so every edge
/// Sample a trim at exactly `n` points, excluding the closing endpoint.
///
/// The next edge in the loop contributes the shared junction, so every edge
/// omits its own end. This is the ONLY sampler: the earlier code had one
/// path that excluded the endpoint and another that popped it afterwards,
/// which differed by one and desynchronised the shared pairing.
fn trim_samples(curve: &axiolid_curve::Curve2, n: usize) -> GeomResult<Vec<axiolid_core::Point2>> {
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

/// A face boundary recognised as a rectangle in parameter space.
///
/// Cylinders, cones, spheres and tori are trimmed by constant-u and
/// constant-v curves in the overwhelmingly common case, and such a patch
/// has a structure a polygon triangulator cannot see. Earcut works in UV,
/// where one unit of u and one unit of v are interchangeable; in 3D they
/// are not. On a cylinder of radius r, a step in u is r times longer than
/// the same step in v, so a triangulation that is perfectly reasonable in
/// parameter space can join a point on the bottom rim to a distant point
/// on the top rim, and that chord cuts THROUGH the tube rather than
/// running along it. The mesh stays closed and its area grows.
///
/// Recognising the rectangle lets the patch be meshed as a grid, where
/// every quad spans exactly one cell and no chord can cut across.
struct GridPatch {
    /// Cells along u and v.
    nu: usize,
    nv: usize,
    u_start: Scalar,
    v_start: Scalar,
    du: Scalar,
    dv: Scalar,
}

/// Collect a curved face's boundary, sampling each edge exactly once across
/// the whole shell.
fn curved_boundary(
    mesh: &mut TriMesh,
    ctx: &FaceContext<'_>,
    welded: &mut std::collections::HashMap<axiolid_topology::VertexId, u32>,
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
            let Some(axiolid_model::GeometryNode::Curve2(trim)) = graph.get(pcurve) else {
                return Err(GeomError::InvalidInput(
                    "edge pcurve must reference a Curve2 node".to_string(),
                ));
            };
            let n = match cache.get(&use_.edge) {
                Some((v, _)) => v.len(),
                None => edge_sample_count(trim, surface, tolerance.linear())?,
            };
            let params = trim_samples(trim, n)?;
            // Evaluate the trim on the surface: these are the seam's
            // 3D points, and they are interned under the edge.
            let points: Vec<Vec3> = params
                .iter()
                .map(|p| axiolid_scalar::surface::evaluate(surface, p.x, p.y))
                .collect::<GeomResult<_>>()?;
            let edge = brep
                .edges()
                .get(use_.edge.index())
                .ok_or_else(|| GeomError::InvalidInput("edge missing".to_string()))?;
            let start_vertex = if use_.orientation == Orientation::Forward {
                edge.start
            } else {
                edge.end
            };
            let shared = edge_samples(
                mesh,
                cache,
                welded,
                use_.edge,
                use_.orientation,
                start_vertex,
                &points,
            );
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
    welded: &mut std::collections::HashMap<axiolid_topology::VertexId, u32>,
    edge: axiolid_topology::EdgeId,
    sense: Orientation,
    start_vertex: axiolid_topology::VertexId,
    points: &[Vec3],
) -> Vec<u32> {
    if let Some((existing, stored)) = cache.get(&edge) {
        // Walk the shared vertices in this use's direction.
        //
        // A use omits its own end vertex, because the next edge in the loop
        // supplies that junction. Reversing therefore cannot be a plain
        // reverse of the stored list: the samples run start..<end, so the
        // reversed walk must BEGIN at the end vertex, which this use's
        // stored samples never contained. On a seam the next edge is this
        // same edge reversed, so that omitted vertex is exactly the one the
        // second use needs first; without it the two uses of a seam start
        // from the same point and the join never pairs.
        if *stored == sense {
            return existing.clone();
        }
        let mut out = Vec::with_capacity(existing.len());
        out.push(*welded.get(&start_vertex).unwrap_or(&existing[0]));
        out.extend(existing.iter().rev().take(existing.len() - 1).copied());
        return out;
    }

    let mut indices: Vec<u32> = Vec::with_capacity(points.len());
    for (i, p) in points.iter().enumerate() {
        if i == 0 {
            let index = *welded.entry(start_vertex).or_insert_with(|| {
                let next = mesh.positions.len() as u32;
                mesh.positions.push(*p);
                next
            });
            indices.push(index);
            continue;
        }
        let next = mesh.positions.len() as u32;
        mesh.positions.push(*p);
        indices.push(next);
    }

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
    welded: &mut std::collections::HashMap<axiolid_topology::VertexId, u32>,
    face: &axiolid_topology::Face<NodeId>,
    surface: &axiolid_surface::Surface,
    cache: &mut EdgeSamples,
    flip: bool,
) -> GeomResult<()> {
    let boundary = curved_boundary(mesh, ctx, welded, face, surface, cache)?;
    if boundary.uv.len() < 3 {
        return Ok(());
    }
    // Prefer a structured grid when the patch is a rectangle in parameter
    // space. Earcut is correct in UV but blind to the metric: it can join a
    // point on one rim to a distant point on the other, and that chord cuts
    // through the solid instead of following the surface. A grid cannot,
    // because every quad spans exactly one cell.
    if let Some(patch) = recognise_grid(&boundary, ctx.tolerance.linear()) {
        let grid = grid_vertices(mesh, &boundary, &patch, surface)?;
        let columns = patch.nu + 1;
        for iv in 0..patch.nv {
            for iu in 0..patch.nu {
                let a = grid[iv * columns + iu];
                let b = grid[iv * columns + iu + 1];
                let c = grid[(iv + 1) * columns + iu + 1];
                let d = grid[(iv + 1) * columns + iu];
                for tri in [[a, b, d], [b, c, d]] {
                    let [x, y, z] = tri;
                    if x == y || y == z || z == x {
                        continue;
                    }
                    if flip {
                        mesh.indices.extend([x, z, y]);
                    } else {
                        mesh.indices.extend([x, y, z]);
                    }
                }
            }
        }
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
        // Drop degenerate triangles: on a periodic face the u = 0 and u = TAU
        // boundary columns map to the SAME 3D vertices, so a trim polygon that
        // is proper in parameter space can produce triangles with a repeated
        // corner. Emitting them leaves self-edges that no manifold check can
        // pair, and they carry no area either way.
        if a == b || b == c || c == a {
            continue;
        }

        if flip {
            mesh.indices.extend([a, c, b]);
        } else {
            mesh.indices.extend([a, b, c]);
        }
    }
    Ok(())
}

/// `recognise_grid` decides which faces get a structured grid. Its
/// guards are load-bearing: accepting a face that is not a clean
/// rectangular lattice would pave over holes or invent geometry, so
/// each rejection is tested directly rather than through mesh area,
/// which earcut and the grid agree on for simple patches.
#[cfg(test)]
mod grid_recognition_tests {
    use super::*;
    use axiolid_core::Point2;

    fn boundary(uv: Vec<Point2>, hole_starts: Vec<usize>) -> CurvedBoundary {
        let shared = (0..uv.len() as u32).collect();
        CurvedBoundary {
            uv,
            shared,
            hole_starts,
        }
    }

    /// A clean 2x1 lattice is recognised, with the step counts read
    /// from the distinct coordinates rather than assumed.
    #[test]
    fn a_rectangular_lattice_is_recognised() {
        let uv = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 1.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let patch = recognise_grid(&boundary(uv, Vec::new()), 1e-9).expect("lattice");
        assert_eq!((patch.nu, patch.nv), (2, 1));
    }

    /// A hole cannot be gridded: the lattice has no way to express it,
    /// so the face must fall back to the triangulator that can.
    #[test]
    fn a_hole_is_refused() {
        let uv = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 1.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
            // A degenerate inner ring whose points all lie on the border:
            // every geometric guard accepts these, so only the hole guard
            // itself can reject this boundary.
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
        ];
        let holed = boundary(uv.clone(), vec![6]);
        assert!(
            recognise_grid(&holed, 1e-9).is_none(),
            "a bounded ring must refuse the grid even when its points lie on the border"
        );
        // Control: the SAME points without the hole marker are a clean
        // lattice, so the refusal above is attributable to the hole alone.
        assert!(recognise_grid(&boundary(uv, Vec::new()), 1e-9).is_some());
    }

    /// Unevenly spaced columns are refused. A uniform grid would move
    /// the middle column to the average position, silently relocating
    /// a vertex the trim curve actually placed elsewhere.
    #[test]
    fn non_uniform_spacing_is_refused() {
        let uv = vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.1, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 1.0),
            Point2::new(0.1, 1.0),
            Point2::new(0.0, 1.0),
        ];
        assert!(recognise_grid(&boundary(uv, Vec::new()), 1e-9).is_none());
    }

    /// A degenerate boundary is refused.
    ///
    /// Three points on one border plus a lone spike is not a lattice in
    /// either direction. Gridding it would invent a rectangle the trim
    /// never described.
    #[test]
    fn a_boundary_with_no_cells_is_refused() {
        let uv = vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(5.0, 0.5),
            Point2::new(0.0, 0.5),
        ];
        assert!(recognise_grid(&boundary(uv, Vec::new()), 1e-9).is_none());
    }

    /// A point off the lattice is refused. Snapping it to the nearest
    /// cell would move a boundary vertex, changing the trimmed shape.
    #[test]
    fn an_off_lattice_point_is_refused() {
        let uv = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 1.0),
            Point2::new(1.0, 0.5),
            Point2::new(0.0, 1.0),
        ];
        assert!(recognise_grid(&boundary(uv, Vec::new()), 1e-9).is_none());
    }
}
