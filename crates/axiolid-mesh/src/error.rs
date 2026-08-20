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
        }
    }
}

impl std::error::Error for MeshValidationError {}
