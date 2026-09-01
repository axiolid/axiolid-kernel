//! Portable scalar plane section of a closed oriented triangle mesh.
//!
//! Topology is classified from the exact binary64 plane equation. Geometry is
//! emitted in the caller's plane frame, with source-edge connectivity preserved;
//! no tolerance-based point welding is used.

use std::collections::{BTreeMap, BTreeSet};

use axiolid_core::{Frame3, Point2, Point3};
use axiolid_kernel::{
    Backend, BackendDescriptor, BackendId, CancellationGranularity, ExecutionOptions,
    ExecutionTarget, GeomError, GeomResult, MeshPlaneSection, ScratchRequirement, SectionContour,
    SectionEvidence, SectionLimits, SectionOutcome, Sign,
};
use axiolid_mesh::TriMesh;

use crate::orient3::orient3d;
use crate::orientation::orient2d;

/// Portable deterministic mesh plane-section oracle.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScalarSection;

impl ScalarSection {
    /// Stable identity for this reference provider.
    pub const ID: BackendId = BackendId::new("scalar-section");

    /// Construct the provider.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Backend for ScalarSection {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(Self::ID, ExecutionTarget::PortableCpu)
    }
}

impl MeshPlaneSection for ScalarSection {
    fn scratch_requirement(&self) -> ScratchRequirement {
        // Signs/distances, source-edge keys, segment adjacency, and cycle state.
        // A closed triangular manifold has O(triangles) vertices and edges.
        ScratchRequirement::PerElement {
            bytes_per_element: 512,
        }
    }

    fn cancellation_granularity(&self) -> CancellationGranularity {
        CancellationGranularity::Incremental
    }

    fn section(
        &self,
        mesh: &TriMesh,
        frame: Frame3,
        limits: SectionLimits,
        options: &ExecutionOptions,
    ) -> GeomResult<SectionOutcome> {
        options.check_cancelled()?;
        check_source_limits(mesh, limits)?;

        let plane = ExactSectionPlane::new(frame)?;
        let mut classifications = Vec::new();
        classifications
            .try_reserve_exact(mesh.positions.len())
            .map_err(|_| GeomError::BudgetExceeded { resource: "memory" })?;
        for &point in &mesh.positions {
            let classification = plane.classify(point)?;
            classifications.push(classification);
        }

        let mut segments = BTreeSet::<Segment>::new();
        let mut on_plane_edges = BTreeMap::<EdgeKey, Vec<Sign>>::new();
        for (triangle_index, triangle) in mesh.triangles().enumerate() {
            options.check_cancelled()?;
            let vertices = [
                mesh_index(triangle[0])?,
                mesh_index(triangle[1])?,
                mesh_index(triangle[2])?,
            ];
            let signs = vertices.map(|index| classifications[index].sign);
            let zero_count = signs.iter().filter(|&&sign| sign == Sign::Zero).count();
            match zero_count {
                3 => {
                    return Err(GeomError::Degenerate(format!(
                        "section plane contains source triangle {triangle_index}; a two-dimensional overlap is not a curve"
                    )))
                }
                2 => {
                    let mut zeros = triangle
                        .into_iter()
                        .zip(signs)
                        .filter_map(|(index, sign)| (sign == Sign::Zero).then_some(index));
                    let left = zeros.next().ok_or_else(internal_topology_error)?;
                    let right = zeros.next().ok_or_else(internal_topology_error)?;
                    let third = signs
                        .into_iter()
                        .find(|&sign| sign != Sign::Zero)
                        .ok_or_else(internal_topology_error)?;
                    on_plane_edges
                        .entry(EdgeKey::new(left, right))
                        .or_default()
                        .push(third);
                }
                1 => {
                    let zero_corner = (0..3)
                        .find(|&corner| signs[corner] == Sign::Zero)
                        .ok_or_else(internal_topology_error)?;
                    let (first, second) = match zero_corner {
                        0 => (1, 2),
                        1 => (0, 2),
                        2 => (0, 1),
                        _ => return Err(internal_topology_error()),
                    };
                    if opposite(signs[first], signs[second]) {
                        let segment = Segment::new(
                            NodeKey::Vertex(triangle[zero_corner]),
                            NodeKey::Edge(EdgeKey::new(triangle[first], triangle[second])),
                        )?;
                        insert_segment(&mut segments, segment, limits)?;
                    }
                }
                0 => {
                    let mut crossing = [None, None];
                    let mut crossing_count = 0usize;
                    for (left, right) in [(0, 1), (1, 2), (2, 0)] {
                        if opposite(signs[left], signs[right]) {
                            if crossing_count >= crossing.len() {
                                return Err(internal_topology_error());
                            }
                            crossing[crossing_count] = Some(NodeKey::Edge(EdgeKey::new(
                                triangle[left],
                                triangle[right],
                            )));
                            crossing_count += 1;
                        }
                    }
                    match (crossing[0], crossing[1]) {
                        (Some(first), Some(second)) => insert_segment(
                            &mut segments,
                            Segment::new(first, second)?,
                            limits,
                        )?,
                        (None, None) => {}
                        _ => return Err(internal_topology_error()),
                    }
                }
                _ => return Err(internal_topology_error()),
            }
        }

        for (edge, incident_signs) in on_plane_edges {
            options.check_cancelled()?;
            if incident_signs.len() != 2 {
                return Err(GeomError::NotManifold(format!(
                    "on-plane mesh edge {:?} has {} incident triangles",
                    edge,
                    incident_signs.len()
                )));
            }
            if opposite(incident_signs[0], incident_signs[1]) {
                insert_segment(
                    &mut segments,
                    Segment::new(NodeKey::Vertex(edge.0), NodeKey::Vertex(edge.1))?,
                    limits,
                )?;
            }
        }

        let contours =
            assemble_contours(mesh, frame, &classifications, &segments, limits, options)?;
        let output_vertices = contours.iter().map(|contour| contour.points.len()).sum();
        let evidence =
            SectionEvidence::input_mesh(mesh.triangle_count(), output_vertices, contours.len());
        Ok(SectionOutcome::new(frame, contours, evidence))
    }
}

#[derive(Debug, Clone, Copy)]
struct Classification {
    sign: Sign,
    distance: f64,
}

#[derive(Debug, Clone, Copy)]
struct ExactSectionPlane {
    origin: Point3,
    x_point: Point3,
    y_point: Point3,
    normal: axiolid_core::Vec3,
}

impl ExactSectionPlane {
    fn new(frame: Frame3) -> GeomResult<Self> {
        let x_point = frame.origin + frame.x;
        let y_point = frame.origin + frame.y;
        if !x_point.is_finite()
            || !y_point.is_finite()
            || x_point == frame.origin
            || y_point == frame.origin
            || x_point == y_point
        {
            return Err(GeomError::Degenerate(
                "section frame cannot resolve a finite affine plane at this coordinate magnitude"
                    .into(),
            ));
        }
        let normal = (x_point - frame.origin).cross(y_point - frame.origin);
        let normal_length = normal.length();
        if !normal_length.is_finite() || normal_length == 0.0 {
            return Err(GeomError::Degenerate(
                "section affine plane has no finite normal".into(),
            ));
        }
        Ok(Self {
            origin: frame.origin,
            x_point,
            y_point,
            normal: normal / normal_length,
        })
    }

    fn classify(self, point: Point3) -> GeomResult<Classification> {
        let sign = match orient3d(self.origin, self.x_point, self.y_point, point) {
            axiolid_kernel::Certified::Certain { sign, .. } => sign,
            _ => {
                return Err(GeomError::Degenerate(
                    "certified plane-side predicate returned an uncertain sign".into(),
                ))
            }
        };
        let distance = self.normal.dot(point - self.origin);
        if !distance.is_finite() {
            return Err(GeomError::Degenerate(
                "section signed distance is not finite".into(),
            ));
        }
        if sign != Sign::Zero && distance == 0.0 {
            return Err(GeomError::Degenerate(
                "section interpolation lost a certified nonzero plane offset".into(),
            ));
        }
        Ok(Classification { sign, distance })
    }
}

fn opposite(left: Sign, right: Sign) -> bool {
    matches!(
        (left, right),
        (Sign::Negative, Sign::Positive) | (Sign::Positive, Sign::Negative)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeKey(u32, u32);

impl EdgeKey {
    fn new(left: u32, right: u32) -> Self {
        Self(left.min(right), left.max(right))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum NodeKey {
    Vertex(u32),
    Edge(EdgeKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Segment(NodeKey, NodeKey);

impl Segment {
    fn new(left: NodeKey, right: NodeKey) -> GeomResult<Self> {
        if left == right {
            return Err(GeomError::Degenerate(
                "plane section collapsed a segment to one source-topology node".into(),
            ));
        }
        Ok(Self(left.min(right), left.max(right)))
    }
}

fn insert_segment(
    segments: &mut BTreeSet<Segment>,
    segment: Segment,
    limits: SectionLimits,
) -> GeomResult<()> {
    if segments.contains(&segment) {
        return Err(GeomError::NotManifold(
            "two source faces produced the same non-coplanar section segment".into(),
        ));
    }
    if segments.len() >= limits.max_output_vertices {
        return Err(GeomError::BudgetExceeded {
            resource: "section output vertices",
        });
    }
    segments.insert(segment);
    Ok(())
}

fn assemble_contours(
    mesh: &TriMesh,
    frame: Frame3,
    classifications: &[Classification],
    segments: &BTreeSet<Segment>,
    limits: SectionLimits,
    options: &ExecutionOptions,
) -> GeomResult<Vec<SectionContour>> {
    let mut adjacency = BTreeMap::<NodeKey, Vec<NodeKey>>::new();
    for &Segment(left, right) in segments {
        adjacency.entry(left).or_default().push(right);
        adjacency.entry(right).or_default().push(left);
    }
    for neighbours in adjacency.values_mut() {
        neighbours.sort_unstable();
        if neighbours.len() != 2 {
            return Err(GeomError::NotManifold(format!(
                "section graph has degree {}, expected 2",
                neighbours.len()
            )));
        }
    }

    let mut visited = BTreeSet::<Segment>::new();
    let mut contours = Vec::new();
    for &start in adjacency.keys() {
        let start_is_complete = adjacency[&start].iter().try_fold(true, |complete, &next| {
            let segment = Segment::new(start, next)?;
            Ok::<bool, GeomError>(complete && visited.contains(&segment))
        })?;
        if start_is_complete {
            continue;
        }
        options.check_cancelled()?;
        if contours.len() >= limits.max_contours {
            return Err(GeomError::BudgetExceeded {
                resource: "section contours",
            });
        }
        let mut nodes = Vec::new();
        let mut previous = None;
        let mut current = start;
        loop {
            if nodes.len() >= limits.max_output_vertices {
                return Err(GeomError::BudgetExceeded {
                    resource: "section output vertices",
                });
            }
            nodes.push(current);
            let neighbours = adjacency
                .get(&current)
                .ok_or_else(internal_topology_error)?;
            let next = match previous {
                None => neighbours[0],
                Some(previous) if neighbours[0] == previous => neighbours[1],
                Some(_) => neighbours[0],
            };
            let edge = Segment::new(current, next)?;
            if !visited.insert(edge) && next != start {
                return Err(GeomError::NotManifold(
                    "section graph revisited an edge before closing a contour".into(),
                ));
            }
            previous = Some(current);
            current = next;
            if current == start {
                break;
            }
            if nodes.len() > segments.len() {
                return Err(internal_topology_error());
            }
        }
        if nodes.len() < 3 {
            return Err(GeomError::Degenerate(
                "section contour has fewer than three source-topology nodes".into(),
            ));
        }
        let mut points = Vec::new();
        points
            .try_reserve_exact(nodes.len())
            .map_err(|_| GeomError::BudgetExceeded { resource: "memory" })?;
        for node in nodes {
            let world = node_point(mesh, classifications, node)?;
            let local = world - frame.origin;
            let point = Point2::new(local.dot(frame.x), local.dot(frame.y));
            if !point.is_finite() {
                return Err(GeomError::Degenerate(
                    "section projection exceeded finite arithmetic".into(),
                ));
            }
            points.push(point);
        }
        simplify_collinear(&mut points, options.tolerance().linear());
        if points.len() < 3 {
            return Err(GeomError::Degenerate(
                "section contour collapsed below three vertices".into(),
            ));
        }
        let area = signed_area(&points);
        if !area.is_finite() || area == 0.0 {
            return Err(GeomError::Degenerate(
                "section contour has no finite signed area".into(),
            ));
        }
        if area < 0.0 {
            points[1..].reverse();
        }
        contours.push(SectionContour::new(points));
    }
    contours.sort_by(|left, right| point_order(&left.points[0], &right.points[0]));
    Ok(contours)
}

fn mesh_index(index: u32) -> GeomResult<usize> {
    usize::try_from(index)
        .map_err(|_| GeomError::InvalidInput("mesh index does not fit usize".into()))
}

fn node_point(
    mesh: &TriMesh,
    classifications: &[Classification],
    node: NodeKey,
) -> GeomResult<Point3> {
    match node {
        NodeKey::Vertex(index) => Ok(mesh.positions[mesh_index(index)?]),
        NodeKey::Edge(EdgeKey(left, right)) => {
            let a = mesh.positions[mesh_index(left)?];
            let b = mesh.positions[mesh_index(right)?];
            let da = classifications[mesh_index(left)?].distance.abs();
            let db = classifications[mesh_index(right)?].distance.abs();
            if !(da > 0.0 && db > 0.0 && da.is_finite() && db.is_finite()) {
                return Err(GeomError::Degenerate(
                    "crossing edge has no representable endpoint distance".into(),
                ));
            }
            let t = if da >= db {
                1.0 / (1.0 + db / da)
            } else {
                let ratio = da / db;
                ratio / (1.0 + ratio)
            };
            let point = a + (b - a) * t;
            if !point.is_finite() {
                return Err(GeomError::Degenerate(
                    "edge-plane intersection exceeded finite arithmetic".into(),
                ));
            }
            Ok(point)
        }
    }
}

fn simplify_collinear(points: &mut Vec<Point2>, tolerance: f64) {
    loop {
        if points.len() <= 3 {
            return;
        }
        let mut removed = false;
        for index in 0..points.len() {
            let previous = points[(index + points.len() - 1) % points.len()];
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            let chord = next - previous;
            let scale = chord.length();
            let cross = (current - previous).perp_dot(chord).abs();
            let within_tolerance = scale > 0.0 && cross <= tolerance * scale;
            if orient2d(previous, current, next).sign() == Some(Sign::Zero) || within_tolerance {
                points.remove(index);
                removed = true;
                break;
            }
        }
        if !removed {
            return;
        }
    }
}

fn signed_area(points: &[Point2]) -> f64 {
    let mut twice = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        twice += current.x * next.y - current.y * next.x;
    }
    twice * 0.5
}

fn point_order(left: &Point2, right: &Point2) -> std::cmp::Ordering {
    left.x
        .total_cmp(&right.x)
        .then_with(|| left.y.total_cmp(&right.y))
}

fn check_source_limits(mesh: &TriMesh, limits: SectionLimits) -> GeomResult<()> {
    if mesh.positions.len() > limits.max_source_vertices {
        return Err(GeomError::BudgetExceeded {
            resource: "section source vertices",
        });
    }
    if mesh.triangle_count() > limits.max_source_triangles {
        return Err(GeomError::BudgetExceeded {
            resource: "section source triangles",
        });
    }
    Ok(())
}

fn internal_topology_error() -> GeomError {
    GeomError::BackendContractViolation {
        backend: ScalarSection::ID,
        detail: "internal section topology state is inconsistent".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_plane_sign_keeps_a_tiny_nonzero_binary64_offset() {
        let frame = Frame3 {
            origin: Point3::ZERO,
            x: Point3::X,
            y: Point3::Y,
            z: Point3::Z,
        };
        let plane = ExactSectionPlane::new(frame).expect("resolvable plane");
        let point = Point3::new(1.0, -1.0, f64::from_bits(1));
        let certified = orient3d(plane.origin, plane.x_point, plane.y_point, point);
        assert!(matches!(
            certified,
            axiolid_kernel::Certified::Certain {
                precision: axiolid_kernel::Precision::Exact,
                ..
            }
        ));
        let positive = plane.classify(point).expect("finite subnormal");
        assert_ne!(positive.sign, Sign::Zero);
    }
}
