#![forbid(unsafe_code)]

//! Exact two-dimensional section profiles.
//!
//! The crate stores profile intent. Boolean cleanup and triangulation are
//! algorithms in higher tiers so a consumer can use profile data without them.

pub mod contour;
pub mod parameterized;
pub mod validate;

use axiolid_core::Transform2;

pub use contour::{Contour, ContourProfile, ProfileSegment};
pub use parameterized::{CircleProfile, EllipseProfile, RectangleProfile, SectionProfile};
pub use validate::ValidateProfile;

/// Format-neutral profile representation.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Profile {
    /// Rectangle or rounded rectangle.
    Rectangle(RectangleProfile),
    /// Circle or annulus.
    Circle(CircleProfile),
    /// Ellipse.
    Ellipse(EllipseProfile),
    /// Structural parameterized section.
    Section(SectionProfile),
    /// Arbitrary exact contour with holes.
    Contour(ContourProfile),
    /// Profile transformed from another profile.
    Derived {
        /// Base profile.
        basis: Box<Profile>,
        /// Two-dimensional transform.
        transform: Transform2,
    },
    /// Ordered collection of profiles used as one section.
    Composite(Vec<Profile>),
}
