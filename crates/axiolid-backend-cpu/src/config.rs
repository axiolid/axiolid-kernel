//! Validated CPU backend builder.

use core::fmt;
use std::num::NonZeroUsize;

use crate::{CpuExecution, CpuFeatures, CpuInstructionSet};

/// Instruction-selection policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionPolicy {
    /// Best runtime-detected path compiled into this crate.
    Auto,
    /// Always portable scalar.
    Portable,
    /// Fail construction unless one path is supported.
    Require(CpuInstructionSet),
}

/// Invalid backend configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuConfigError {
    /// Required instruction set is absent on this machine.
    UnsupportedInstructionSet(CpuInstructionSet),
    /// More than one thread was requested without the `parallel` feature.
    ParallelFeatureDisabled,
    /// Local Rayon pool construction failed.
    ThreadPool(String),
}

impl fmt::Display for CpuConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedInstructionSet(value) => {
                write!(f, "required CPU instruction set {value:?} is unavailable")
            }
            Self::ParallelFeatureDisabled => {
                f.write_str("multiple workers require axiolid-backend-cpu's `parallel` feature")
            }
            Self::ThreadPool(reason) => {
                write!(f, "failed to build local CPU thread pool: {reason}")
            }
        }
    }
}

impl std::error::Error for CpuConfigError {}

/// Builder for a validated backend bound to current runtime capabilities.
#[derive(Debug, Clone, Copy)]
pub struct CpuExecutionBuilder {
    instruction_policy: InstructionPolicy,
    threads: NonZeroUsize,
}

impl Default for CpuExecutionBuilder {
    fn default() -> Self {
        Self {
            instruction_policy: InstructionPolicy::Auto,
            threads: NonZeroUsize::MIN,
        }
    }
}

impl CpuExecutionBuilder {
    /// New portable-safe builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Choose instruction policy.
    pub const fn instruction_policy(mut self, policy: InstructionPolicy) -> Self {
        self.instruction_policy = policy;
        self
    }

    /// Set a nonzero worker bound.
    pub const fn threads(mut self, threads: NonZeroUsize) -> Self {
        self.threads = threads;
        self
    }

    /// Detect hardware, validate policy, and construct the backend.
    pub fn build(self) -> Result<CpuExecution, CpuConfigError> {
        let features = CpuFeatures::detect();
        let instruction_set = match self.instruction_policy {
            InstructionPolicy::Portable => CpuInstructionSet::Portable,
            InstructionPolicy::Auto => {
                #[cfg(feature = "simd")]
                {
                    features.best()
                }
                #[cfg(not(feature = "simd"))]
                {
                    CpuInstructionSet::Portable
                }
            }
            InstructionPolicy::Require(value) => {
                if value != CpuInstructionSet::Portable && !cfg!(feature = "simd") {
                    return Err(CpuConfigError::UnsupportedInstructionSet(value));
                }
                if !features.supports(value) {
                    return Err(CpuConfigError::UnsupportedInstructionSet(value));
                }
                value
            }
        };
        CpuExecution::from_configuration(instruction_set, features, self.threads)
    }
}
