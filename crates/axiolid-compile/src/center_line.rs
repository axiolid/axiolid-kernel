//! Centre-line profiles: offset an open path into a closed boundary.
//!
//! The area a centre line denotes is the set of points within a half-width of
//! its path. This module resolves that into an explicit ring by walking the
//! flattened path out one side and back the other.
//!
//! Offsetting is done on the FLATTENED path rather than on the exact curves.
//! A true exact offset of a spline is a different curve type, not the same
//! curve moved sideways, so producing one here would either be wrong or would
//! need a curve representation the kernel does not have. Flattening first
//! keeps the approximation in one place and under the caller's chord budget,
//! which is the same contract every other curved profile already accepts.

use axiolid_core::{Point2, Scalar, Tolerance};
use axiolid_kernel::{GeomError, GeomResult};
use axiolid_profile::CenterLineProfile;

use crate::profile::Rings;

/// Resolve a centre-line profile into a closed ring.
pub fn center_line_rings(
    profile: &CenterLineProfile,
    chord_error: Scalar,
    tolerance: Tolerance,
    flatten: impl Fn(&axiolid_profile::Contour, Scalar, Tolerance) -> GeomResult<Vec<Point2>>,
) -> GeomResult<Rings> {
    // Written as an explicit non-positive test rather than a negated `>`:
    // a NaN half-width must be refused too, and `!(x > 0.0)` states that by
    // accident rather than on purpose.
    if profile.half_width <= 0.0 || profile.half_width.is_nan() {
        return Err(GeomError::Degenerate(format!(
            "centre line half-width must be positive, got {}",
            profile.half_width
        )));
    }
    let path = flatten(&profile.path, chord_error, tolerance)?;
    if path.len() < 2 {
        return Err(GeomError::Degenerate(format!(
            "centre line path flattened to {} points, need at least 2",
            path.len()
        )));
    }

    let left = offset_polyline(&path, profile.half_width, tolerance)?;
    let right = offset_polyline(&path, -profile.half_width, tolerance)?;

    // Walk out along one side and back along the other: the two offsets plus
    // the flat end caps form one closed ring. Butt caps are used because the
    // source states a width and an extent, not an end treatment; inventing a
    // round or square cap would add material the author did not specify.
    let mut outer = left;
    outer.extend(right.into_iter().rev());
    Ok(Rings {
        outer,
        holes: Vec::new(),
    })
}

/// Offset a polyline sideways by a signed distance.
///
/// Interior vertices use a MITER join: the offset point sits on the
/// intersection of the two offset edges, which is the only join that keeps a
/// constant width through a corner. Bevelling or rounding a corner would make
/// the section narrower there than the author declared.
///
/// The miter length grows without bound as a corner closes on itself, so a
/// reversal is rejected rather than emitting a spike that would self-intersect
/// the ring and produce a solid whose volume depends on the tessellation.
fn offset_polyline(
    path: &[Point2],
    distance: Scalar,
    tolerance: Tolerance,
) -> GeomResult<Vec<Point2>> {
    let eps = tolerance.linear();
    let normal_of = |a: Point2, b: Point2| -> GeomResult<Point2> {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= eps {
            return Err(GeomError::Degenerate(
                "centre line has a zero-length segment".to_string(),
            ));
        }
        // Left normal of the direction vector.
        Ok(Point2::new(-dy / len, dx / len))
    };

    let mut out = Vec::with_capacity(path.len());
    for index in 0..path.len() {
        if index == 0 {
            let n = normal_of(path[0], path[1])?;
            out.push(Point2::new(
                path[0].x + n.x * distance,
                path[0].y + n.y * distance,
            ));
        } else if index == path.len() - 1 {
            let n = normal_of(path[index - 1], path[index])?;
            out.push(Point2::new(
                path[index].x + n.x * distance,
                path[index].y + n.y * distance,
            ));
        } else {
            let n0 = normal_of(path[index - 1], path[index])?;
            let n1 = normal_of(path[index], path[index + 1])?;
            // The miter direction bisects the two edge normals; scaling it by
            // 1/cos(half-angle) puts it on both offset lines at once.
            let mx = n0.x + n1.x;
            let my = n0.y + n1.y;
            let denom = 1.0 + (n0.x * n1.x + n0.y * n1.y);
            if denom <= eps {
                return Err(GeomError::Degenerate(
                    "centre line reverses on itself; the miter is unbounded".to_string(),
                ));
            }
            out.push(Point2::new(
                path[index].x + (mx / denom) * distance,
                path[index].y + (my / denom) * distance,
            ));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiolid_core::Interval;
    use axiolid_curve::linear::Polyline2;
    use axiolid_curve::Curve2;
    use axiolid_profile::{Contour, ProfileSegment};

    fn polyline_path(points: &[(f64, f64)]) -> Contour {
        let pts: Vec<Point2> = points.iter().map(|(x, y)| Point2::new(*x, *y)).collect();
        Contour::new(vec![ProfileSegment {
            curve: Curve2::Polyline(Polyline2 {
                points: pts,
                closed: false,
            }),
            domain: Interval::new(0.0, (points.len() - 1) as f64),
            same_sense: true,
        }])
    }

    fn flatten(contour: &Contour, _chord: Scalar, _tol: Tolerance) -> GeomResult<Vec<Point2>> {
        // The real flattener closes rings by dropping a repeated last point.
        // A centre line is open, so the test feeds points through unchanged.
        let mut out = Vec::new();
        for segment in &contour.segments {
            if let Curve2::Polyline(p) = &segment.curve {
                out.extend(p.points.iter().copied());
            }
        }
        Ok(out)
    }

    fn tol() -> Tolerance {
        Tolerance::new(1e-9, 1e-9).expect("valid tolerance")
    }

    fn area(ring: &[Point2]) -> f64 {
        let mut sum = 0.0;
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            sum += a.x * b.y - b.x * a.y;
        }
        sum.abs() / 2.0
    }

    /// A straight centre line encloses length times width, exactly.
    ///
    /// This is the assertion that catches a half-width read as a full width:
    /// the ring still closes and still looks like a plausible bar, but every
    /// quantity taken from it is out by a factor of two.
    #[test]
    fn straight_center_line_has_length_times_width_area() {
        let profile = CenterLineProfile::from_width(polyline_path(&[(0.0, 0.0), (2.0, 0.0)]), 0.05);
        let rings = center_line_rings(
            &profile,
            1e-4,
            Tolerance::new(1e-9, 1e-9).expect("valid tolerance"),
            flatten,
        )
        .expect("straight centre line resolves");
        assert!(rings.holes.is_empty(), "a centre line encloses no holes");
        let got = area(&rings.outer);
        assert!(
            (got - 0.1).abs() < 1e-12,
            "2.0 long by 0.05 wide is 0.1, got {got}"
        );
    }

    /// A right-angle corner keeps full width through the bend.
    ///
    /// A mitered strip has area equal to centre-line length times width,
    /// because the outer corner gains exactly the wedge the inner corner
    /// loses. That identity is what makes this test worth having: it holds for
    /// ANY corner angle, so it catches a join that bevels (too little area) or
    /// one that overshoots the miter (too much), neither of which a
    /// closes-and-looks-plausible check would notice.
    ///
    /// It is deliberately NOT the union of two bars minus their overlap
    /// (0.19): that describes a strip whose corner is clipped square, which is
    /// a different and narrower section through the bend.
    #[test]
    fn a_mitered_corner_keeps_constant_width() {
        let profile = CenterLineProfile::from_width(
            polyline_path(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]),
            0.1,
        );
        let rings = center_line_rings(&profile, 1e-4, tol(), flatten).expect("corner resolves");
        let got = area(&rings.outer);
        assert!(
            (got - 0.2).abs() < 1e-12,
            "a mitered strip is length times width: 2.0 * 0.1 = 0.2, got {got}"
        );
    }

    /// Half the width is measured each side, not the whole width.
    #[test]
    fn width_is_symmetric_about_the_path() {
        let profile = CenterLineProfile::from_width(polyline_path(&[(0.0, 0.0), (1.0, 0.0)]), 0.2);
        let rings = center_line_rings(&profile, 1e-4, tol(), flatten).expect("resolves");
        let ys: Vec<f64> = rings.outer.iter().map(|p| p.y).collect();
        let top = ys.iter().cloned().fold(f64::MIN, f64::max);
        let bottom = ys.iter().cloned().fold(f64::MAX, f64::min);
        assert!(
            (top - 0.1).abs() < 1e-12,
            "top offset is half the width, got {top}"
        );
        assert!(
            (bottom + 0.1).abs() < 1e-12,
            "bottom offset is half the width, got {bottom}"
        );
    }

    /// A path that doubles back has an unbounded miter and is refused.
    #[test]
    fn a_reversing_path_is_refused_not_spiked() {
        let profile = CenterLineProfile::from_width(
            polyline_path(&[(0.0, 0.0), (1.0, 0.0), (0.0, 0.0)]),
            0.1,
        );
        let error = center_line_rings(&profile, 1e-4, tol(), flatten)
            .expect_err("a reversal has no finite miter");
        assert!(
            matches!(error, GeomError::Degenerate(_)),
            "expected a typed degeneracy, got {error:?}"
        );
    }

    /// A non-positive width encloses nothing and is refused.
    #[test]
    fn a_zero_width_center_line_is_refused() {
        let profile = CenterLineProfile::from_width(polyline_path(&[(0.0, 0.0), (1.0, 0.0)]), 0.0);
        let error = center_line_rings(&profile, 1e-4, tol(), flatten)
            .expect_err("zero width encloses no area");
        assert!(matches!(error, GeomError::Degenerate(_)));
    }

    /// A NaN width is refused rather than propagating into every coordinate.
    ///
    /// NaN fails every comparison, so a naive positive-width guard lets it
    /// through and it silently poisons the whole ring.
    #[test]
    fn a_nan_width_center_line_is_refused() {
        let profile = CenterLineProfile {
            path: polyline_path(&[(0.0, 0.0), (1.0, 0.0)]),
            half_width: f64::NAN,
        };
        let error =
            center_line_rings(&profile, 1e-4, tol(), flatten).expect_err("NaN is not a width");
        assert!(matches!(error, GeomError::Degenerate(_)));
    }
}
