//! Differential properties of regular parametric surfaces.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point3, Scalar, Tolerance, Vec3};
use axiolid_reference::surface::jet;
use axiolid_surface::Surface;

/// Coefficients `(e, f, g)` of a quadratic fundamental form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FundamentalForm {
    /// First diagonal coefficient (`E` for the first form, `e` for the second).
    pub e: Scalar,
    /// Mixed coefficient (`F` for the first form, `f` for the second).
    pub f: Scalar,
    /// Second diagonal coefficient (`G` for the first form, `g` for the second).
    pub g: Scalar,
}

/// First/second-order differential properties of an oriented surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceDifferential {
    /// Evaluated surface point.
    pub point: Point3,
    /// Unit normal from `du × dv`.
    pub unit_normal: Vec3,
    /// First fundamental form `(E, F, G)`.
    pub first: FundamentalForm,
    /// Second fundamental form `(e, f, g)` for `unit_normal`.
    pub second: FundamentalForm,
    /// Gaussian curvature, invariant under normal reversal.
    pub gaussian_curvature: Scalar,
    /// Mean curvature for the reported normal orientation.
    pub mean_curvature: Scalar,
    /// Principal curvatures in ascending order for the reported orientation.
    pub principal_curvatures: [Scalar; 2],
}

/// Analyze a regular oriented surface at `(u, v)`.
pub fn analyze_surface(
    surface: &Surface,
    u: Scalar,
    v: Scalar,
    tolerance: Tolerance,
) -> GeomResult<SurfaceDifferential> {
    let j = jet(surface, u, v)?;
    let cross = j.du.cross(j.dv);
    let area = cross.length();
    let area_floor = tolerance.linear() * tolerance.linear();
    if !area.is_finite() || area <= area_floor {
        return Err(GeomError::Degenerate(format!(
            "surface differential area {area} does not exceed the tolerance area"
        )));
    }
    let n = cross / area;
    let first = FundamentalForm {
        e: j.du.dot(j.du),
        f: j.du.dot(j.dv),
        g: j.dv.dot(j.dv),
    };
    let second = FundamentalForm {
        e: n.dot(j.duu),
        f: n.dot(j.duv),
        g: n.dot(j.dvv),
    };
    let determinant = first.e * first.g - first.f * first.f;
    if !determinant.is_finite() || determinant <= area_floor * area_floor {
        return Err(GeomError::Degenerate(
            "surface first fundamental form is singular".to_owned(),
        ));
    }
    let gaussian = (second.e * second.g - second.f * second.f) / determinant;
    let mean =
        (second.e * first.g - 2.0 * second.f * first.f + second.g * first.e) / (2.0 * determinant);
    let raw_discriminant = mean * mean - gaussian;
    let scale = mean.abs().max(gaussian.abs().sqrt()).max(1.0);
    let floor = -64.0 * Scalar::EPSILON * scale * scale;
    if raw_discriminant < floor || !raw_discriminant.is_finite() {
        return Err(GeomError::Degenerate(
            "surface principal-curvature discriminant is invalid".to_owned(),
        ));
    }
    let root = raw_discriminant.max(0.0).sqrt();
    let result = SurfaceDifferential {
        point: j.point,
        unit_normal: n,
        first,
        second,
        gaussian_curvature: gaussian,
        mean_curvature: mean,
        principal_curvatures: [mean - root, mean + root],
    };
    if [
        gaussian,
        mean,
        result.principal_curvatures[0],
        result.principal_curvatures[1],
    ]
    .iter()
    .all(|x| x.is_finite())
    {
        Ok(result)
    } else {
        Err(GeomError::Degenerate(
            "surface curvature is non-finite".to_owned(),
        ))
    }
}
