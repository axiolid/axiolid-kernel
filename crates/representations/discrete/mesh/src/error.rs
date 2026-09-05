//! Structured mesh validation failures.

use core::fmt;

/// Cheap structural validation failure.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshValidationError {
    /// Triangle index buffer is not divisible by three.
    IncompleteTriangle { index_count: usize },
    /// Position index does not exist.
    PositionIndexOutOfRange { index: u32, position_count: usize },
    /// Non-indexed normals do not align with positions.
    NormalCount { expected: usize, actual: usize },
    /// Independently indexed normals do not align with corners.
    NormalIndexCount { expected: usize, actual: usize },
    /// Normal index does not exist.
    NormalIndexOutOfRange { index: u32, normal_count: usize },
    /// A channel does not carry one tuple per vertex.
    AttributeCount {
        /// Channel name.
        name: String,
        /// Vertices in the mesh.
        expected: usize,
        /// Vertices the channel covers.
        actual: usize,
    },
    /// A channel declares a zero tuple width, which covers no vertices.
    AttributeZeroWidth {
        /// Channel name.
        name: String,
    },
    /// Two channels share a name, so a lookup would be ambiguous.
    AttributeDuplicateName {
        /// The repeated name.
        name: String,
    },
}

impl fmt::Display for MeshValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompleteTriangle { index_count } => {
                write!(f, "index count {index_count} is not divisible by three")
            }
            Self::PositionIndexOutOfRange {
                index,
                position_count,
            } => write!(
                f,
                "position index {index} exceeds {position_count} positions"
            ),
            Self::NormalCount { expected, actual } => {
                write!(f, "expected {expected} normals, found {actual}")
            }
            Self::NormalIndexCount { expected, actual } => {
                write!(f, "expected {expected} normal indices, found {actual}")
            }
            Self::NormalIndexOutOfRange {
                index,
                normal_count,
            } => write!(f, "normal index {index} exceeds {normal_count} normals"),
            Self::AttributeCount {
                name,
                expected,
                actual,
            } => write!(
                f,
                "attribute channel {name} covers {actual} vertices, mesh has {expected}"
            ),
            Self::AttributeZeroWidth { name } => {
                write!(f, "attribute channel {name} declares a zero tuple width")
            }
            Self::AttributeDuplicateName { name } => {
                write!(f, "attribute channel name {name} is used more than once")
            }
        }
    }
}

impl std::error::Error for MeshValidationError {}
