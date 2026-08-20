//! Mesh boolean capability and executable provider registry.

use std::sync::Arc;

use axiolid_core::BooleanOperator;
use axiolid_mesh::TriMesh;

use crate::{
    Backend, BackendId, DevicePreference, ExecutionOptions, ExecutionTarget, GeomError, GeomResult,
    Operation, ScratchRequirement,
};

/// Mesh boolean provider.
///
/// Implementing this trait is the capability declaration. Providers that do not
/// implement mesh booleans must not implement this trait.
pub trait MeshBoolean: Backend {
    /// Scratch this provider needs beyond its inputs and result.
    ///
    /// Callers budget against this before dispatch. Defaults to
    /// [`ScratchRequirement::Unbounded`] so an unaudited provider is treated as
    /// unbudgetable rather than silently assumed cheap.
    fn scratch_requirement(&self) -> ScratchRequirement {
        ScratchRequirement::Unbounded
    }

    /// Apply one set operation.
    fn boolean(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        operation: BooleanOperator,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh>;

    /// Subtract many tools in one batch so implementations can union or schedule
    /// cutters efficiently. The default is correct but deliberately simple.
    fn subtract_many(
        &self,
        subject: &TriMesh,
        tools: &[TriMesh],
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let mut result = subject.clone();
        for tool in tools {
            result = self.boolean(&result, tool, BooleanOperator::Difference, options)?;
        }
        Ok(result)
    }
}

#[derive(Debug, Clone)]
struct RegisteredBoolean {
    priority: i32,
    provider: Arc<dyn MeshBoolean>,
}

/// Ordered executable providers for one narrow operation.
///
/// Fallback happens only for `Unsupported` or `Unavailable`; numerical and data
/// failures are returned immediately rather than hidden by another algorithm.
#[derive(Debug, Clone, Default)]
pub struct MeshBooleanRegistry {
    providers: Vec<RegisteredBoolean>,
}

impl MeshBooleanRegistry {
    /// Empty registry.
    pub const fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register an implementation. Higher priorities run first.
    pub fn register<B>(&mut self, priority: i32, provider: B)
    where
        B: MeshBoolean + 'static,
    {
        self.register_arc(priority, Arc::new(provider));
    }

    /// Register a shared trait object.
    pub fn register_arc(&mut self, priority: i32, provider: Arc<dyn MeshBoolean>) {
        self.providers
            .push(RegisteredBoolean { priority, provider });
        self.providers
            .sort_by_key(|entry| std::cmp::Reverse(entry.priority));
    }

    /// Registered providers in dispatch order.
    pub fn providers(&self) -> impl Iterator<Item = &dyn MeshBoolean> {
        self.providers.iter().map(|entry| entry.provider.as_ref())
    }

    fn dispatch(
        &self,
        options: &ExecutionOptions,
        elements: usize,
        execute: impl Fn(&dyn MeshBoolean) -> GeomResult<TriMesh>,
    ) -> GeomResult<TriMesh> {
        let mut last_retryable = None;
        let mut over_budget = None;
        for entry in &self.providers {
            let descriptor = entry.provider.descriptor();
            if !matches_device(options.device(), descriptor.id, descriptor.target) {
                continue;
            }
            // Budget is checked before dispatch, not after: a provider that
            // cannot fit the caller's memory bound must never get the chance to
            // allocate. Treated as retryable so a leaner provider can still run.
            if !entry
                .provider
                .scratch_requirement()
                .fits_budget(options, elements)
            {
                over_budget = Some(GeomError::BudgetExceeded { resource: "memory" });
                continue;
            }
            match execute(entry.provider.as_ref()) {
                Ok(mesh) => return Ok(mesh),
                Err(error @ (GeomError::Unsupported { .. } | GeomError::Unavailable { .. })) => {
                    last_retryable = Some(error);
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_retryable
            .or(over_budget)
            .unwrap_or(GeomError::Unsupported {
                backend: BackendId::new("mesh-boolean-registry"),
                operation: Operation::MeshBoolean,
            }))
    }

    /// Execute according to device policy with narrow fallback semantics.
    pub fn boolean(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        operation: BooleanOperator,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        self.dispatch(options, subject.triangle_count(), |provider| {
            provider.boolean(subject, tool, operation, options)
        })
    }

    /// Subtract many tools through one provider dispatch.
    pub fn subtract_many(
        &self,
        subject: &TriMesh,
        tools: &[TriMesh],
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let elements =
            subject.triangle_count() + tools.iter().map(TriMesh::triangle_count).sum::<usize>();
        self.dispatch(options, elements, |provider| {
            provider.subtract_many(subject, tools, options)
        })
    }
}

fn matches_device(preference: DevicePreference, id: BackendId, target: ExecutionTarget) -> bool {
    match preference {
        DevicePreference::Auto => true,
        DevicePreference::Cpu => {
            matches!(
                target,
                ExecutionTarget::PortableCpu | ExecutionTarget::OptimizedCpu
            )
        }
        DevicePreference::Gpu => matches!(target, ExecutionTarget::Gpu),
        DevicePreference::Backend(required) => required == id,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use axiolid_core::Tolerance;

    use super::*;
    use crate::BackendDescriptor;

    #[derive(Debug)]
    struct EchoBoolean {
        id: BackendId,
        target: ExecutionTarget,
    }

    impl Backend for EchoBoolean {
        fn descriptor(&self) -> BackendDescriptor {
            BackendDescriptor {
                id: self.id,
                target: self.target,
            }
        }
    }

    impl MeshBoolean for EchoBoolean {
        fn boolean(
            &self,
            subject: &TriMesh,
            _tool: &TriMesh,
            _operation: BooleanOperator,
            _options: &ExecutionOptions,
        ) -> GeomResult<TriMesh> {
            Ok(subject.clone())
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum ProbeResult {
        Success,
        Unsupported,
        Unavailable,
        Invalid,
    }

    #[derive(Debug)]
    struct ProbeBoolean {
        id: BackendId,
        target: ExecutionTarget,
        result: ProbeResult,
        calls: Arc<AtomicUsize>,
    }

    impl Backend for ProbeBoolean {
        fn descriptor(&self) -> BackendDescriptor {
            BackendDescriptor::new(self.id, self.target)
        }
    }

    impl MeshBoolean for ProbeBoolean {
        fn boolean(
            &self,
            subject: &TriMesh,
            _tool: &TriMesh,
            _operation: BooleanOperator,
            _options: &ExecutionOptions,
        ) -> GeomResult<TriMesh> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            match self.result {
                ProbeResult::Success => Ok(subject.clone()),
                ProbeResult::Unsupported => Err(GeomError::Unsupported {
                    backend: self.id,
                    operation: Operation::MeshBoolean,
                }),
                ProbeResult::Unavailable => Err(GeomError::Unavailable {
                    backend: self.id,
                    reason: "probe unavailable".to_owned(),
                }),
                ProbeResult::Invalid => {
                    Err(GeomError::InvalidInput("probe rejected input".to_owned()))
                }
            }
        }
    }

    #[derive(Debug)]
    struct BatchBoolean {
        calls: Arc<AtomicUsize>,
    }

    impl Backend for BatchBoolean {
        fn descriptor(&self) -> BackendDescriptor {
            BackendDescriptor::new(BackendId::new("batch"), ExecutionTarget::OptimizedCpu)
        }
    }

    impl MeshBoolean for BatchBoolean {
        fn boolean(
            &self,
            _subject: &TriMesh,
            _tool: &TriMesh,
            _operation: BooleanOperator,
            _options: &ExecutionOptions,
        ) -> GeomResult<TriMesh> {
            Err(GeomError::InvalidInput(
                "batch provider must use its batch override".to_owned(),
            ))
        }

        fn subtract_many(
            &self,
            subject: &TriMesh,
            _tools: &[TriMesh],
            _options: &ExecutionOptions,
        ) -> GeomResult<TriMesh> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(subject.clone())
        }
    }

    #[test]
    fn registry_stores_executable_traits_not_capability_flags() {
        let mut registry = MeshBooleanRegistry::new();
        registry.register(
            10,
            EchoBoolean {
                id: BackendId::new("echo"),
                target: ExecutionTarget::PortableCpu,
            },
        );
        let options = ExecutionOptions::new(Tolerance::METRE);
        let mesh = TriMesh::default();
        assert_eq!(
            registry
                .boolean(&mesh, &mesh, BooleanOperator::Difference, &options)
                .expect("registered provider executes"),
            mesh
        );
    }

    #[test]
    fn registry_dispatches_batch_subtraction_to_the_provider_override() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = MeshBooleanRegistry::new();
        registry.register(
            10,
            BatchBoolean {
                calls: calls.clone(),
            },
        );
        let mesh = TriMesh::default();
        let options = ExecutionOptions::new(Tolerance::METRE);

        assert_eq!(
            registry
                .subtract_many(&mesh, &[mesh.clone(), mesh.clone()], &options)
                .expect("batch provider executes"),
            mesh
        );
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn registry_falls_back_only_for_retryable_errors_and_honors_device_policy() {
        let high_calls = Arc::new(AtomicUsize::new(0));
        let unsupported_calls = Arc::new(AtomicUsize::new(0));
        let low_calls = Arc::new(AtomicUsize::new(0));
        let mut registry = MeshBooleanRegistry::new();
        registry.register(
            100,
            ProbeBoolean {
                id: BackendId::new("unavailable-gpu"),
                target: ExecutionTarget::Gpu,
                result: ProbeResult::Unavailable,
                calls: high_calls.clone(),
            },
        );
        registry.register(
            50,
            ProbeBoolean {
                id: BackendId::new("unsupported-cpu"),
                target: ExecutionTarget::OptimizedCpu,
                result: ProbeResult::Unsupported,
                calls: unsupported_calls.clone(),
            },
        );
        registry.register(
            10,
            ProbeBoolean {
                id: BackendId::new("portable-fallback"),
                target: ExecutionTarget::PortableCpu,
                result: ProbeResult::Success,
                calls: low_calls.clone(),
            },
        );
        let mesh = TriMesh::default();
        let auto = ExecutionOptions::new(Tolerance::METRE);
        assert!(registry
            .boolean(&mesh, &mesh, BooleanOperator::Union, &auto)
            .is_ok());
        assert_eq!(high_calls.load(Ordering::Relaxed), 1);
        assert_eq!(unsupported_calls.load(Ordering::Relaxed), 1);
        assert_eq!(low_calls.load(Ordering::Relaxed), 1);

        let cpu = auto.with_device(DevicePreference::Cpu);
        assert!(registry
            .boolean(&mesh, &mesh, BooleanOperator::Union, &cpu)
            .is_ok());
        assert_eq!(high_calls.load(Ordering::Relaxed), 1);
        assert_eq!(unsupported_calls.load(Ordering::Relaxed), 2);
        assert_eq!(low_calls.load(Ordering::Relaxed), 2);

        let invalid_calls = Arc::new(AtomicUsize::new(0));
        let skipped_calls = Arc::new(AtomicUsize::new(0));
        let mut fail_fast = MeshBooleanRegistry::new();
        fail_fast.register(
            100,
            ProbeBoolean {
                id: BackendId::new("invalid-input"),
                target: ExecutionTarget::PortableCpu,
                result: ProbeResult::Invalid,
                calls: invalid_calls.clone(),
            },
        );
        fail_fast.register(
            10,
            ProbeBoolean {
                id: BackendId::new("must-not-run"),
                target: ExecutionTarget::PortableCpu,
                result: ProbeResult::Success,
                calls: skipped_calls.clone(),
            },
        );
        assert!(matches!(
            fail_fast.boolean(&mesh, &mesh, BooleanOperator::Union, &auto),
            Err(GeomError::InvalidInput(_))
        ));
        assert_eq!(invalid_calls.load(Ordering::Relaxed), 1);
        assert_eq!(skipped_calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn empty_registry_returns_structured_unsupported_error() {
        let registry = MeshBooleanRegistry::new();
        let mesh = TriMesh::default();
        let error = registry
            .boolean(
                &mesh,
                &mesh,
                BooleanOperator::Union,
                &ExecutionOptions::new(Tolerance::METRE),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            GeomError::Unsupported {
                backend,
                operation: Operation::MeshBoolean,
            } if backend == BackendId::new("mesh-boolean-registry")
        ));
    }
}
