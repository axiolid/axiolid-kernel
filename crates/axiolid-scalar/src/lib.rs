#![forbid(unsafe_code)]

//! Portable scalar reference implementation: the correctness oracle (ADR 0012).
//!
//! No intrinsics, no threading, no feature gates. Every optimized backend is
//! validated by differential test against this crate, so it must stay readable
//! and obviously correct in preference to being fast.

pub mod arithmetic;
pub mod boolean;
pub mod convex_hull;
pub mod curve;
pub mod expansion;
pub mod orient3;
pub mod orientation;
pub mod polygon;
pub mod scene;
pub mod segment_triangle;
pub mod sphere;
pub mod static_filter;
pub mod triangle_triangle;

pub use boolean::ScalarBoolean;
pub use convex_hull::{minimum_area_rectangle, strict_convex_hull, OrientedRectangle2};
pub use curve::{derivative2, derivative3, evaluate2, evaluate3, flatten2, ScalarCurve};
pub use expansion::{two_diff, two_product, two_sum};
pub use orient3::{orient3d, orient3d_filter};
pub use orientation::{orient2d, orient2d_filter};
pub use polygon::{ring_orientation, signed_area2, triangulate_simple};
pub use segment_triangle::{segment_triangle_relation, SegmentTriangleRelation};
pub use sphere::{incircle, incircle_filter, insphere, insphere_filter};
pub use static_filter::StaticFilter;
pub use triangle_triangle::{triangle_triangle_relation, TriangleTriangleRelation};
