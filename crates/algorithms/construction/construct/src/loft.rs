//! Lofting a sequence of station rings into a solid.
//!
//! Every sweep family reduces to the same shape: place the profile at a
//! series of stations along a path, stitch consecutive stations into
//! walls, and cap the ends unless the path closes on itself. Extrusion is
//! two stations; revolution is an arc of them; a sectioned spine supplies
//! its own.
//!
//! Writing that once means winding, hole orientation and cap pairing are
//! fixed in one place rather than re-derived per family.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point2, Point3, Scalar, Vec3};
use axiolid_mesh::TriMesh;

use crate::profile::Rings;

/// One station: the profile's rings already placed in 3D.
pub struct Station {
    /// Outer ring followed by each hole, in `Rings` order.
    pub loops: Vec<Vec<Point3>>,
}

/// Place `rings` into 3D with a per-point mapping.
///
/// The mapping takes the profile's 2D point and returns its 3D image, so a
/// caller expresses only what its family does: extrusion translates,
/// revolution rotates, a spine sweep applies a frame.
pub fn place(rings: &Rings, mut f: impl FnMut(Point2) -> Point3) -> Station {
    let mut loops = Vec::with_capacity(1 + rings.holes.len());
    loops.push(rings.outer.iter().map(|p| f(*p)).collect());
    for hole in &rings.holes {
        loops.push(hole.iter().map(|p| f(*p)).collect());
    }
    Station { loops }
}

/// Stitch stations into a closed solid.
///
/// `closed` wraps the last station onto the first, which welds a periodic
/// seam by index rather than by two samplings agreeing numerically. Open
/// lofts are capped with the profile triangulation at each end.
///
/// All stations must share the profile's ring structure: the walls pair
/// vertices by position, so a differing ring length has no meaningful
/// pairing and is refused rather than silently truncated.
pub fn loft(rings: &Rings, stations: &[Station], closed: bool) -> GeomResult<TriMesh> {
    if stations.len() < 2 {
        return Err(GeomError::InvalidInput(format!(
            "a loft needs at least two stations, got {}",
            stations.len()
        )));
    }
    let shape: Vec<usize> = stations[0].loops.iter().map(|r| r.len()).collect();
    for s in stations {
        let this: Vec<usize> = s.loops.iter().map(|r| r.len()).collect();
        if this != shape {
            return Err(GeomError::InvalidInput(
                "loft stations must share the profile's ring structure".to_owned(),
            ));
        }
    }
    let per_station: usize = shape.iter().sum();
    let mut positions: Vec<Point3> = Vec::with_capacity(stations.len() * per_station);
    for s in stations {
        for ring in &s.loops {
            positions.extend(ring.iter().copied());
        }
    }
    // A closed loft wraps onto station 0; an open one stops one short.
    let spans = if closed {
        stations.len()
    } else {
        stations.len() - 1
    };
    let mut indices: Vec<u32> = Vec::new();
    for s in 0..spans {
        let a0 = s * per_station;
        let b0 = ((s + 1) % stations.len()) * per_station;
        let mut base = 0usize;
        for m in shape.iter() {
            for k in 0..*m {
                let kn = (k + 1) % *m;
                let (a, b) = ((a0 + base + k) as u32, (a0 + base + kn) as u32);
                let (c, d) = ((b0 + base + k) as u32, (b0 + base + kn) as u32);
                // Both rings wind the same way here. A hole ring is already
                // stored clockwise (profile.rs keeps holes reversed), which
                // is what turns its wall normal inward; flipping the index
                // order as well would invert it a second time.
                indices.extend([a, b, d, a, d, c]);
            }
            base += *m;
        }
    }
    // Caps. A closed loft needs none: the wall meets itself. An open one is
    // bounded by the profile at each end, wound opposite so both face out.
    if !closed {
        let (points, tris) = crate::profile::triangulate(rings)?;
        if points.len() != per_station {
            return Err(GeomError::Degenerate(format!(
                "cap triangulation produced {} points for {per_station} ring points",
                points.len()
            )));
        }
        let last = ((stations.len() - 1) * per_station) as u32;
        for t in &tris {
            indices.extend([t[0], t[2], t[1]]);
            indices.extend([last + t[0], last + t[1], last + t[2]]);
        }
    }
    Ok(TriMesh::new(positions, indices))
}

/// An orthonormal frame at a point on a path.
///
/// A sweep needs a full frame, not just a tangent: the profile's x and y
/// axes have to be carried along the path, and how they are carried is
/// exactly what distinguishes the sweep families from each other.
pub struct Frame {
    /// Frame origin on the directrix.
    pub origin: Point3,
    /// Image of the profile's +x.
    pub x: Vec3,
    /// Image of the profile's +y.
    pub y: Vec3,
}

impl Frame {
    /// Frame whose z is `tangent` and whose x is `reference` made
    /// perpendicular to it.
    ///
    /// Refuses a reference parallel to the tangent instead of silently
    /// picking a fallback axis: the caller supplied a direction that cannot
    /// orient the profile, and quietly substituting one rotates the section
    /// by an arbitrary angle.
    pub fn from_reference(origin: Point3, tangent: Vec3, reference: Vec3) -> GeomResult<Self> {
        let t = tangent.normalize_or_zero();
        if t == Vec3::ZERO {
            return Err(GeomError::InvalidInput(
                "sweep tangent must be a non-zero direction".to_owned(),
            ));
        }
        let x = (reference - t * t.dot(reference)).normalize_or_zero();
        if x == Vec3::ZERO {
            return Err(GeomError::InvalidInput(
                "sweep reference direction must not be parallel to the directrix".to_owned(),
            ));
        }
        Ok(Self {
            origin,
            x,
            y: t.cross(x),
        })
    }
}

/// Place a profile point using a frame.
pub fn at(frame: &Frame, p: Point2) -> Point3 {
    frame.origin + frame.x * p.x + frame.y * p.y
}

/// Linear blend of two profile rings.
///
/// Tapered families interpolate between a start and an end profile, so the
/// blend belongs here rather than in each of them.
pub fn blend(a: Point2, b: Point2, t: Scalar) -> Point2 {
    Point2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// Loft where the two ends carry different profiles.
///
/// [`loft`] caps both ends from one ring set, which is wrong the moment the
/// ends differ: a tapered solid needs each cap triangulated from its own
/// profile. The walls are identical, so only the caps are rebuilt here.
pub fn loft_tapered(start: &Rings, end: &Rings, stations: &[Station]) -> GeomResult<TriMesh> {
    let mut mesh = loft(start, stations, false)?;
    let per_station: usize = stations[0].loops.iter().map(|r| r.len()).sum();
    // Drop the caps `loft` built from `start` and rebuild the far one from
    // `end`. Wall triangles come first, so truncating to the wall count is
    // exact rather than a search.
    let spans = stations.len() - 1;
    let ring_edges: usize = stations[0].loops.iter().map(|r| r.len()).sum();
    let wall_indices = spans * ring_edges * 6;
    mesh.indices.truncate(wall_indices);
    let (near_pts, near_tris) = crate::profile::triangulate(start)?;
    let (far_pts, far_tris) = crate::profile::triangulate(end)?;
    if near_pts.len() != per_station || far_pts.len() != per_station {
        return Err(GeomError::Degenerate(
            "tapered cap triangulation disagrees with the station rings".to_owned(),
        ));
    }
    let last = (spans * per_station) as u32;
    for t in &near_tris {
        mesh.indices.extend([t[0], t[2], t[1]]);
    }
    for t in &far_tris {
        mesh.indices.extend([last + t[0], last + t[1], last + t[2]]);
    }
    Ok(mesh)
}
