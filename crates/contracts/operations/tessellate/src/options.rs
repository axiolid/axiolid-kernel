//! Explicit tessellation quality policy.

use axiolid_core::{Scalar, Tolerance};

/// Invalid tessellation options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTessellationOptions;

impl core::fmt::Display for InvalidTessellationOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("tessellation limits must be finite and positive")
    }
}

impl std::error::Error for InvalidTessellationOptions {}

/// Approximation controls. No global/default chord error exists because source
/// units and downstream use determine acceptable error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TessellationOptions {
    chord_error: Scalar,
    maximum_angle: Scalar,
    maximum_edge_length: Option<Scalar>,
    tolerance: Tolerance,
}

impl TessellationOptions {
    /// Construct validated approximation controls.
    pub fn new(
        chord_error: Scalar,
        maximum_angle: Scalar,
        tolerance: Tolerance,
    ) -> Result<Self, InvalidTessellationOptions> {
        if !chord_error.is_finite()
            || chord_error <= 0.0
            || !maximum_angle.is_finite()
            || maximum_angle <= 0.0
        {
            return Err(InvalidTessellationOptions);
        }
        Ok(Self {
            chord_error,
            maximum_angle,
            maximum_edge_length: None,
            tolerance,
        })
    }

    /// Add a maximum edge length.
    pub fn with_maximum_edge_length(
        mut self,
        value: Scalar,
    ) -> Result<Self, InvalidTessellationOptions> {
        if !value.is_finite() || value <= 0.0 {
            return Err(InvalidTessellationOptions);
        }
        self.maximum_edge_length = Some(value);
        Ok(self)
    }

    /// Maximum chord deviation.
    pub const fn chord_error(self) -> Scalar {
        self.chord_error
    }

    /// Maximum change in tangent/normal in radians.
    pub const fn maximum_angle(self) -> Scalar {
        self.maximum_angle
    }

    /// Optional maximum generated edge length.
    pub const fn maximum_edge_length(self) -> Option<Scalar> {
        self.maximum_edge_length
    }

    /// Structural/evaluation tolerance.
    pub const fn tolerance(self) -> Tolerance {
        self.tolerance
    }
}
