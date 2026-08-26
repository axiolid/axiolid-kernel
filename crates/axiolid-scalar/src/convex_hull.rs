//! Deterministic 2D convex hulls and oriented bounding rectangles.
use crate::orient2d;
use axiolid_core::{Point2, Scalar};
use axiolid_kernel::{GeomError, GeomResult, Sign};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedRectangle2 {
    corners: [Point2; 4],
    area: Scalar,
    sides: [Scalar; 2],
}
impl OrientedRectangle2 {
    #[must_use]
    pub fn corners(self) -> [Point2; 4] {
        self.corners
    }
    #[must_use]
    pub fn area(self) -> Scalar {
        self.area
    }
    #[must_use]
    pub fn side_lengths(self) -> [Scalar; 2] {
        self.sides
    }
}

pub fn strict_convex_hull(points: &[Point2]) -> GeomResult<Vec<usize>> {
    for (index, point) in points.iter().enumerate() {
        if !point.is_finite() {
            return Err(GeomError::InvalidInput(format!(
                "point {index} is not finite"
            )));
        }
    }
    let mut ordered: Vec<usize> = (0..points.len()).collect();
    ordered.sort_by(|&a, &b| {
        points[a]
            .x
            .total_cmp(&points[b].x)
            .then_with(|| points[a].y.total_cmp(&points[b].y))
            .then(a.cmp(&b))
    });
    ordered.dedup_by(|a, b| points[*a] == points[*b]);
    if ordered.len() < 3 {
        return Err(GeomError::Degenerate("need three distinct points".into()));
    }
    let mut lower = Vec::new();
    for &index in &ordered {
        push_strict(&mut lower, index, points);
    }
    let mut upper = Vec::new();
    for &index in ordered.iter().rev() {
        push_strict(&mut upper, index, points);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    if lower.len() < 3 {
        return Err(GeomError::Degenerate("points are collinear".into()));
    }
    Ok(lower)
}

pub fn minimum_area_rectangle(points: &[Point2]) -> GeomResult<OrientedRectangle2> {
    let hull = strict_convex_hull(points)?;
    let mut best: Option<OrientedRectangle2> = None;
    for i in 0..hull.len() {
        let a = points[hull[i]];
        let b = points[hull[(i + 1) % hull.len()]];
        let edge = b - a;
        let width = edge.length();
        let u = edge / width;
        let v = Point2::new(-u.y, u.x);
        let (mut ulo, mut uhi, mut vlo, mut vhi) = (
            Scalar::INFINITY,
            Scalar::NEG_INFINITY,
            Scalar::INFINITY,
            Scalar::NEG_INFINITY,
        );
        for &index in &hull {
            let point = points[index];
            let pu = point.dot(u);
            let pv = point.dot(v);
            ulo = ulo.min(pu);
            uhi = uhi.max(pu);
            vlo = vlo.min(pv);
            vhi = vhi.max(pv);
        }
        let sides = [uhi - ulo, vhi - vlo];
        let area = sides[0] * sides[1];
        let make = |x, y| u * x + v * y;
        let rectangle = OrientedRectangle2 {
            corners: [
                make(ulo, vlo),
                make(uhi, vlo),
                make(uhi, vhi),
                make(ulo, vhi),
            ],
            area,
            sides,
        };
        if best.as_ref().is_none_or(|current| area < current.area) {
            best = Some(rectangle);
        }
    }
    Ok(best.expect("non-empty strict hull has an edge"))
}

fn push_strict(hull: &mut Vec<usize>, index: usize, points: &[Point2]) {
    while hull.len() >= 2 {
        let n = hull.len();
        if sign(orient2d(
            points[hull[n - 2]],
            points[hull[n - 1]],
            points[index],
        )) == Sign::Positive
        {
            break;
        }
        hull.pop();
    }
    hull.push(index);
}
fn sign(value: axiolid_kernel::Certified) -> Sign {
    value.sign().expect("orient2d is total")
}
