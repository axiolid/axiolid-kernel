//! Compact parameterized profile families.

use axiolid_core::Scalar;

/// Rectangle, optionally rounded at the corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangleProfile {
    /// Extent along local x.
    pub x: Scalar,
    /// Extent along local y.
    pub y: Scalar,
    /// Optional wall thickness. `None` denotes a filled section.
    pub thickness: Option<Scalar>,
    /// Optional outer corner radius.
    pub outer_radius: Option<Scalar>,
    /// Optional inner corner radius for hollow sections.
    pub inner_radius: Option<Scalar>,
}

/// Circle or annulus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleProfile {
    /// Outer radius.
    pub radius: Scalar,
    /// Optional wall thickness. `None` denotes a filled disk.
    pub thickness: Option<Scalar>,
}

/// Ellipse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipseProfile {
    /// Semi-axis along local x.
    pub semi_axis_x: Scalar,
    /// Semi-axis along local y.
    pub semi_axis_y: Scalar,
}

/// Generic structural section dimensions.
///
/// Source adapters map I, L, T, U, C, Z and trapezium profile entities into a
/// named variant plus dimensions, preserving optional fillet/slope values.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum SectionProfile {
    /// I or asymmetric I section.
    I {
        depth: Scalar,
        width: Scalar,
        web_thickness: Scalar,
        flange_thickness: Scalar,
    },
    /// L section.
    L {
        depth: Scalar,
        width: Scalar,
        thickness: Scalar,
    },
    /// T section.
    T {
        depth: Scalar,
        width: Scalar,
        web_thickness: Scalar,
        flange_thickness: Scalar,
    },
    /// U or channel section.
    U {
        depth: Scalar,
        width: Scalar,
        web_thickness: Scalar,
        flange_thickness: Scalar,
    },
    /// C section.
    C {
        depth: Scalar,
        width: Scalar,
        wall_thickness: Scalar,
        girth: Scalar,
    },
    /// Z section.
    Z {
        depth: Scalar,
        flange_width: Scalar,
        web_thickness: Scalar,
        flange_thickness: Scalar,
    },
    /// Trapezium.
    Trapezium {
        bottom_x: Scalar,
        top_x: Scalar,
        y: Scalar,
        top_offset: Scalar,
    },
}
