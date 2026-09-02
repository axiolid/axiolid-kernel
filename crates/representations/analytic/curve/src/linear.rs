//! Linear curve values.
//!
//! These types moved to the focused `axiolid-linear` package so a line-query
//! consumer can compile them without the general curve aggregate. They are
//! re-exported here unchanged, so `axiolid_curve::Line2` and
//! `axiolid_linear::Line2` remain the same type rather than two lookalikes.

pub use axiolid_linear::{Line, Line2, Line3, Polyline, Polyline2, Polyline3};
