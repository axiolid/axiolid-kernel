//! Runtime CPU execution context. Operation providers may compose this type.

use core::fmt;
use std::num::NonZeroUsize;
#[cfg(feature = "parallel")]
use std::sync::Arc;

use crate::{CpuConfigError, CpuFeatures, CpuInstructionSet};

/// CPU execution context with runtime ISA detection and an optional local pool.
///
/// This type deliberately implements no geometry operation trait. A provider
/// only implements `MeshBoolean`, `GeometryCompiler`, or another capability
/// after the algorithm exists and is verified.
#[derive(Clone)]
pub struct CpuExecution {
    instruction_set: CpuInstructionSet,
    features: CpuFeatures,
    threads: NonZeroUsize,
    #[cfg(feature = "parallel")]
    pool: Option<Arc<rayon::ThreadPool>>,
}

impl fmt::Debug for CpuExecution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpuExecution")
            .field("instruction_set", &self.instruction_set)
            .field("features", &self.features)
            .field("threads", &self.threads)
            .finish_non_exhaustive()
    }
}

impl CpuExecution {
    /// Always-available portable execution context.
    pub fn portable() -> Self {
        Self {
            instruction_set: CpuInstructionSet::Portable,
            features: CpuFeatures::detect(),
            threads: NonZeroUsize::MIN,
            #[cfg(feature = "parallel")]
            pool: None,
        }
    }

    /// Best instruction path compiled into this binary and available at runtime.
    pub fn detect() -> Self {
        let features = CpuFeatures::detect();
        #[cfg(feature = "simd")]
        let instruction_set = features.best();
        #[cfg(not(feature = "simd"))]
        let instruction_set = CpuInstructionSet::Portable;
        Self {
            instruction_set,
            features,
            threads: NonZeroUsize::MIN,
            #[cfg(feature = "parallel")]
            pool: None,
        }
    }

    pub(crate) fn from_configuration(
        instruction_set: CpuInstructionSet,
        features: CpuFeatures,
        threads: NonZeroUsize,
    ) -> Result<Self, CpuConfigError> {
        #[cfg(not(feature = "parallel"))]
        if threads.get() > 1 {
            return Err(CpuConfigError::ParallelFeatureDisabled);
        }

        #[cfg(feature = "parallel")]
        let pool = if threads.get() > 1 {
            Some(Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads.get())
                    .thread_name(|index| format!("axiolid-cpu-{index}"))
                    .build()
                    .map_err(|error| CpuConfigError::ThreadPool(error.to_string()))?,
            ))
        } else {
            None
        };

        Ok(Self {
            instruction_set,
            features,
            threads,
            #[cfg(feature = "parallel")]
            pool,
        })
    }

    /// Runtime-selected instruction set.
    pub const fn instruction_set(&self) -> CpuInstructionSet {
        self.instruction_set
    }

    /// Detected machine features.
    pub const fn features(&self) -> CpuFeatures {
        self.features
    }

    /// Configured worker bound.
    pub const fn thread_count(&self) -> NonZeroUsize {
        self.threads
    }

    /// Execute one closure inside this context's local Rayon pool.
    #[cfg(feature = "parallel")]
    pub fn install<R: Send>(&self, operation: impl FnOnce() -> R + Send) -> R {
        match &self.pool {
            Some(pool) => pool.install(operation),
            None => operation(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CpuExecutionBuilder, InstructionPolicy};

    fn assert_runtime_traits<T: fmt::Debug + Clone + Send + Sync>() {}

    #[test]
    fn portable_context_is_constructible_but_claims_no_operation_trait() {
        assert_runtime_traits::<CpuExecution>();
        let backend = CpuExecutionBuilder::new()
            .instruction_policy(InstructionPolicy::Portable)
            .build()
            .expect("portable context");
        assert_eq!(backend.instruction_set(), CpuInstructionSet::Portable);
    }

    #[cfg(not(feature = "simd"))]
    #[test]
    fn auto_selection_stays_portable_without_the_simd_feature() {
        assert_eq!(
            CpuExecution::detect().instruction_set(),
            CpuInstructionSet::Portable
        );
    }

    #[cfg(feature = "simd")]
    #[test]
    fn auto_selection_uses_the_best_runtime_supported_instruction_set() {
        let backend = CpuExecution::detect();
        assert_eq!(backend.instruction_set(), backend.features().best());
        assert!(backend.features().supports(backend.instruction_set()));
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn configured_worker_bound_controls_the_local_pool() {
        let backend = CpuExecutionBuilder::new()
            .threads(NonZeroUsize::new(2).unwrap())
            .build()
            .expect("two-worker context");
        assert_eq!(backend.thread_count().get(), 2);
        assert_eq!(backend.install(rayon::current_num_threads), 2);
    }

    #[cfg(not(feature = "parallel"))]
    #[test]
    fn multiple_workers_require_the_parallel_feature() {
        let error = CpuExecutionBuilder::new()
            .threads(NonZeroUsize::new(2).unwrap())
            .build()
            .unwrap_err();
        assert_eq!(error, CpuConfigError::ParallelFeatureDisabled);
    }
}
