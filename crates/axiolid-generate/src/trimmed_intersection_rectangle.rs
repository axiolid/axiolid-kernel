use axiolid_brep::SurfaceId;
use axiolid_core::{Point2, Point3};
use axiolid_kernel::{GeomError, GeomResult};
use axiolid_surface::BSplineSurface;
use axiolid_topology::{EdgeId, FaceId, Orientation, VertexId};

use crate::trimmed_intersection_builder::{allocation_error, edge_use, ArrangementBuilder};
use crate::trimmed_intersection_classify::{boundary_rank, Domain2, Endpoint2};

#[derive(Debug, Clone, Copy)]
struct Node {
    rank: f64,
    uv: Point2,
    point: Point3,
    vertex: VertexId,
    endpoint: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
struct BoundaryEdge {
    edge: EdgeId,
    pcurve: axiolid_brep::Curve2Id,
}

pub(super) fn add_unsplit_face(
    builder: &mut ArrangementBuilder,
    surface: &BSplineSurface,
    support: SurfaceId,
    domain: Domain2,
) -> GeomResult<FaceId> {
    let (uvs, points) = rectangle_data(surface, domain)?;
    let vertices = [
        builder.add_vertex(points[0]),
        builder.add_vertex(points[1]),
        builder.add_vertex(points[2]),
        builder.add_vertex(points[3]),
    ];
    let mut uses = Vec::new();
    uses.try_reserve_exact(4)
        .map_err(|_| allocation_error("trimmed rectangle loop allocation"))?;
    for index in 0..4 {
        let next = (index + 1) % 4;
        let edge =
            builder.add_line_edge(vertices[index], vertices[next], points[index], points[next])?;
        let pcurve = builder.add_pcurve(uvs[index], uvs[next])?;
        uses.push(edge_use(edge, Orientation::Forward, pcurve));
    }
    let loop_id = builder.add_loop(uses)?;
    builder.add_face(loop_id, support)
}

pub(super) struct SplitFaces {
    pub faces: [FaceId; 2],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn add_split_faces(
    builder: &mut ArrangementBuilder,
    surface: &BSplineSurface,
    support: SurfaceId,
    domain: Domain2,
    start: Endpoint2,
    end: Endpoint2,
    start_point: Point3,
    end_point: Point3,
    start_vertex: VertexId,
    end_vertex: VertexId,
    intersection_edge: EdgeId,
) -> GeomResult<SplitFaces> {
    let start_side = start.side.ok_or_else(|| ownership_error("start"))?;
    let end_side = end.side.ok_or_else(|| ownership_error("end"))?;
    let (corner_uvs, corner_points) = rectangle_data(surface, domain)?;
    let corner_vertices = [
        builder.add_vertex(corner_points[0]),
        builder.add_vertex(corner_points[1]),
        builder.add_vertex(corner_points[2]),
        builder.add_vertex(corner_points[3]),
    ];
    let mut nodes = [
        Node {
            rank: 0.0,
            uv: corner_uvs[0],
            point: corner_points[0],
            vertex: corner_vertices[0],
            endpoint: None,
        },
        Node {
            rank: 1.0,
            uv: corner_uvs[1],
            point: corner_points[1],
            vertex: corner_vertices[1],
            endpoint: None,
        },
        Node {
            rank: 2.0,
            uv: corner_uvs[2],
            point: corner_points[2],
            vertex: corner_vertices[2],
            endpoint: None,
        },
        Node {
            rank: 3.0,
            uv: corner_uvs[3],
            point: corner_points[3],
            vertex: corner_vertices[3],
            endpoint: None,
        },
        Node {
            rank: boundary_rank(start_side, start.uv, domain),
            uv: start.uv,
            point: start_point,
            vertex: start_vertex,
            endpoint: Some(true),
        },
        Node {
            rank: boundary_rank(end_side, end.uv, domain),
            uv: end.uv,
            point: end_point,
            vertex: end_vertex,
            endpoint: Some(false),
        },
    ];
    nodes.sort_by(|left, right| left.rank.total_cmp(&right.rank));
    let start_index = endpoint_index(&nodes, true)?;
    let end_index = endpoint_index(&nodes, false)?;

    let mut boundary = Vec::new();
    boundary
        .try_reserve_exact(6)
        .map_err(|_| allocation_error("trimmed split boundary allocation"))?;
    for index in 0..6 {
        let next = (index + 1) % 6;
        let edge = builder.add_line_edge(
            nodes[index].vertex,
            nodes[next].vertex,
            nodes[index].point,
            nodes[next].point,
        )?;
        let pcurve = builder.add_pcurve(nodes[index].uv, nodes[next].uv)?;
        boundary.push(BoundaryEdge { edge, pcurve });
    }

    let first = add_split_face(
        builder,
        &nodes,
        &boundary,
        start_index,
        end_index,
        intersection_edge,
        Orientation::Reversed,
        support,
    )?;
    let second = add_split_face(
        builder,
        &nodes,
        &boundary,
        end_index,
        start_index,
        intersection_edge,
        Orientation::Forward,
        support,
    )?;
    Ok(SplitFaces {
        faces: [first, second],
    })
}

#[allow(clippy::too_many_arguments)]
fn add_split_face(
    builder: &mut ArrangementBuilder,
    nodes: &[Node; 6],
    boundary: &[BoundaryEdge],
    from: usize,
    to: usize,
    chord: EdgeId,
    chord_orientation: Orientation,
    support: SurfaceId,
) -> GeomResult<FaceId> {
    let mut uses = Vec::new();
    uses.try_reserve_exact(7)
        .map_err(|_| allocation_error("trimmed split loop allocation"))?;
    let mut index = from;
    let mut traversed = 0usize;
    while index != to {
        if traversed >= 6 {
            return Err(GeomError::Degenerate("split boundary did not close".into()));
        }
        let boundary_use = boundary[index];
        uses.push(edge_use(
            boundary_use.edge,
            Orientation::Forward,
            boundary_use.pcurve,
        ));
        index = (index + 1) % 6;
        traversed += 1;
    }
    let chord_pcurve = builder.add_pcurve(nodes[to].uv, nodes[from].uv)?;
    uses.push(edge_use(chord, chord_orientation, chord_pcurve));
    let loop_id = builder.add_loop(uses)?;
    builder.add_face(loop_id, support)
}

fn endpoint_index(nodes: &[Node; 6], start: bool) -> GeomResult<usize> {
    nodes
        .iter()
        .position(|node| node.endpoint == Some(start))
        .ok_or_else(|| GeomError::Degenerate("split endpoint was lost during ordering".into()))
}

fn rectangle_data(
    surface: &BSplineSurface,
    domain: Domain2,
) -> GeomResult<([Point2; 4], [Point3; 4])> {
    let last_u =
        surface.control_points.len().checked_sub(1).ok_or_else(|| {
            GeomError::InvalidInput("split surface has no control-point rows".into())
        })?;
    let first_row = surface.control_points.first().ok_or_else(|| {
        GeomError::InvalidInput("split surface has no first control-point row".into())
    })?;
    let last_v = first_row
        .len()
        .checked_sub(1)
        .ok_or_else(|| GeomError::InvalidInput("split surface has no control points".into()))?;
    let last_row = surface
        .control_points
        .get(last_u)
        .ok_or_else(|| GeomError::InvalidInput("split surface control net is incomplete".into()))?;
    let points = [
        *first_row
            .first()
            .ok_or_else(|| GeomError::InvalidInput("empty split row".into()))?,
        *last_row
            .first()
            .ok_or_else(|| GeomError::InvalidInput("empty split row".into()))?,
        *last_row
            .get(last_v)
            .ok_or_else(|| GeomError::InvalidInput("ragged split net".into()))?,
        *first_row
            .get(last_v)
            .ok_or_else(|| GeomError::InvalidInput("ragged split net".into()))?,
    ];
    let uvs = [
        Point2::new(domain.u_start, domain.v_start),
        Point2::new(domain.u_end, domain.v_start),
        Point2::new(domain.u_end, domain.v_end),
        Point2::new(domain.u_start, domain.v_end),
    ];
    Ok((uvs, points))
}

fn ownership_error(endpoint: &'static str) -> GeomError {
    GeomError::Degenerate(format!("split {endpoint} endpoint is not boundary-owned"))
}
