#![forbid(unsafe_code)]

//! Portable scalar reference implementation: the correctness oracle (ADR 0012).
//!
//! No intrinsics, no threading, no feature gates. Every optimized backend is
//! validated by differential test against this crate, so it must stay readable
//! and obviously correct in preference to being fast.
//!
//! This package is a **convenience umbrella** (ADR 0036). Certified predicates
//! now live in the focused `axiolid-predicates` package and are re-exported
//! here unchanged, so an existing `axiolid_reference::orient2d` caller is
//! unaffected while a narrow consumer can depend on the substrate directly
//! instead of acquiring this package's whole dependency graph.

pub mod boolean;
pub mod clash;
pub mod convex_hull;
pub mod curve;
mod nurbs;
pub mod polygon;
pub mod primitive;
pub mod section;
pub mod segment_triangle;
pub mod surface;
pub mod tessellate;
pub mod triangle_triangle;

pub use axiolid_predicates::{
    arithmetic, expansion, orient3, orientation, scene, sphere, static_filter,
};

pub use axiolid_predicates::{
    incircle, incircle_filter, insphere, insphere_filter, orient2d, orient2d_filter, orient3d,
    orient3d_filter, two_diff, two_product, two_sum, StaticFilter,
};
pub use boolean::ScalarBoolean;
pub use convex_hull::{minimum_area_rectangle, strict_convex_hull, OrientedRectangle2};
pub use curve::{derivative2, derivative3, evaluate2, evaluate3, flatten2, ScalarCurve};
pub use polygon::{ring_orientation, signed_area2, triangulate_simple};
pub use section::ScalarSection;
pub use segment_triangle::{segment_triangle_relation, SegmentTriangleRelation};
pub use surface::{partials, Patch, ScalarSurface};
pub use triangle_triangle::{triangle_triangle_relation, TriangleTriangleRelation};
