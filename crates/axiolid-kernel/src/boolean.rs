//! Mesh boolean capability and executable provider registry.

use std::sync::Arc;

use axiolid_core::BooleanOperator;
use axiolid_mesh::TriMesh;

use crate::{
    Backend, BackendId, BooleanEvidence, BooleanOutcome, CancellationGranularity, DevicePreference,
    ExecutionOptions, ExecutionTarget, GeomError, GeomResult, Operation, ScratchRequirement,
    SolidRequirements,
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

    /// How finely this provider polls a cancellation token.
    ///
    /// Defaults to [`CancellationGranularity::None`]: a provider that has not
    /// declared otherwise is assumed not to poll. Claiming responsiveness a
    /// provider does not have is worse than admitting none.
    fn cancellation_granularity(&self) -> CancellationGranularity {
        CancellationGranularity::None
    }

    /// Admissibility this provider requires of its operands.
    ///
    /// Advisory only: the registry validates at the contract level before
    /// dispatch. A provider declaring a *lower* level does not thereby get to
    /// accept looser input, and one declaring a higher level is rejected by the
    /// conformance suite for narrowing the contract.
    fn solid_requirements(&self) -> SolidRequirements {
        SolidRequirements::Oriented
    }

    /// Apply one regularized set operation.
    ///
    /// Operands are pre-validated by the registry. Returns a
    /// [`BooleanOutcome`]: the mesh plus what was done to produce it. An empty
    /// result mesh is a legitimate value, not an error.
    fn boolean(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        operation: BooleanOperator,
        options: &ExecutionOptions,
    ) -> GeomResult<BooleanOutcome>;

    /// Subtract many tools in one batch so implementations can union or schedule
    /// cutters efficiently. The default is correct but deliberately simple.
    ///
    /// The default polls cancellation between tools, which is why the default
    /// granularity for an overriding provider must be declared honestly.
    fn subtract_many(
        &self,
        subject: &TriMesh,
        tools: &[TriMesh],
        options: &ExecutionOptions,
    ) -> GeomResult<BooleanOutcome> {
        let mut evidence = BooleanEvidence {
            subject_triangles: subject.triangle_count(),
            tool_triangles: tools.iter().map(TriMesh::triangle_count).sum(),
            output_triangles: subject.triangle_count(),
            output_components: 1,
            ..BooleanEvidence::default()
        };
        let mut result = subject.clone();
        for tool in tools {
            options.check_cancelled()?;
            let outcome = self.boolean(&result, tool, BooleanOperator::Difference, options)?;
            evidence.absorb(outcome.evidence);
            result = outcome.mesh;
        }
        Ok(BooleanOutcome::new(result, evidence))
    }
}

/// Compose `A △ B` as `(A ∪ B) \ (A ∩ B)`.
///
/// Free-standing rather than a trait default so a provider cannot accidentally
/// inherit a composed implementation while reporting `sub_operations: 1`. A
/// native implementor overrides [`MeshBoolean::boolean`] and never calls this.
///
/// Composition is the reason `BooleanEvidence::sub_operations` exists: without
/// it a caller cannot tell a three-pass emulation from a single-pass primitive,
/// and the two have materially different numerical behaviour.
pub fn symmetric_difference_via_composition<P>(
    provider: &P,
    subject: &TriMesh,
    tool: &TriMesh,
    options: &ExecutionOptions,
) -> GeomResult<BooleanOutcome>
where
    P: MeshBoolean + ?Sized,
{
    options.check_cancelled()?;
    let union = provider.boolean(subject, tool, BooleanOperator::Union, options)?;
    options.check_cancelled()?;
    let intersection = provider.boolean(subject, tool, BooleanOperator::Intersection, options)?;

    // A ∩ B empty means the operands are disjoint, so A △ B == A ∪ B. Skipping
    // the final difference is not just an optimisation: subtracting an empty
    // solid is a degenerate operand many backends reject.
    if intersection.mesh.indices.is_empty() {
        let mut evidence = union.evidence;
        evidence.sub_operations = 2;
        evidence.coincident_faces_encountered |= intersection.evidence.coincident_faces_encountered;
        return Ok(BooleanOutcome::new(union.mesh, evidence));
    }

    options.check_cancelled()?;
    let difference = provider.boolean(
        &union.mesh,
        &intersection.mesh,
        BooleanOperator::Difference,
        options,
    )?;

    let mut evidence = difference.evidence;
    evidence.subject_triangles = subject.triangle_count();
    evidence.tool_triangles = tool.triangle_count();
    evidence.sub_operations = 3;
    evidence.coincident_faces_encountered |= union.evidence.coincident_faces_encountered
        || intersection.evidence.coincident_faces_encountered;
    Ok(BooleanOutcome::new(difference.mesh, evidence))
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
        execute: impl Fn(&dyn MeshBoolean) -> GeomResult<BooleanOutcome>,
    ) -> GeomResult<BooleanOutcome> {
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
                Ok(outcome) => return Ok(outcome),
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
    ) -> GeomResult<BooleanOutcome> {
        // Admissibility is contract-level and checked before any provider sees
        // the operands, so dispatch cannot change which inputs are legal.
        SolidRequirements::Oriented.validate_operands(subject, &[tool])?;
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
    ) -> GeomResult<BooleanOutcome> {
        let borrowed: Vec<&TriMesh> = tools.iter().collect();
        SolidRequirements::Oriented.validate_operands(subject, &borrowed)?;
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
    /// Outward-oriented unit cube: the minimal admissible operand.
    ///
    /// Dispatch tests need a mesh that passes contract validation, because
    /// validation now runs before any provider is consulted.
    fn admissible_cube() -> TriMesh {
        let positions = vec![
            [0.0, 0.0, 0.0].into(),
            [1.0, 0.0, 0.0].into(),
            [1.0, 1.0, 0.0].into(),
            [0.0, 1.0, 0.0].into(),
            [0.0, 0.0, 1.0].into(),
            [1.0, 0.0, 1.0].into(),
            [1.0, 1.0, 1.0].into(),
            [0.0, 1.0, 1.0].into(),
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ];
        TriMesh::new(positions, indices)
    }

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
        ) -> GeomResult<BooleanOutcome> {
            Ok(BooleanOutcome::new(
                subject.clone(),
                BooleanEvidence::default(),
            ))
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
        ) -> GeomResult<BooleanOutcome> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            match self.result {
                ProbeResult::Success => Ok(BooleanOutcome::new(
                    subject.clone(),
                    BooleanEvidence::default(),
                )),
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
        ) -> GeomResult<BooleanOutcome> {
            Err(GeomError::InvalidInput(
                "batch provider must use its batch override".to_owned(),
            ))
        }

        fn subtract_many(
            &self,
            subject: &TriMesh,
            _tools: &[TriMesh],
            _options: &ExecutionOptions,
        ) -> GeomResult<BooleanOutcome> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(BooleanOutcome::new(
                subject.clone(),
                BooleanEvidence::default(),
            ))
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
        let mesh = admissible_cube();
        assert_eq!(
            registry
                .boolean(&mesh, &mesh, BooleanOperator::Difference, &options)
                .expect("registered provider executes"),
            BooleanOutcome::new(mesh, BooleanEvidence::default())
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
        let mesh = admissible_cube();
        let options = ExecutionOptions::new(Tolerance::METRE);

        assert_eq!(
            registry
                .subtract_many(&mesh, &[mesh.clone(), mesh.clone()], &options)
                .expect("batch provider executes"),
            BooleanOutcome::new(mesh, BooleanEvidence::default())
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
        let mesh = admissible_cube();
        let auto = ExecutionOptions::new(Tolerance::METRE);
        assert!(registry
            .boolean(&mesh, &mesh, BooleanOperator::Union, &auto)
            .is_ok());
        assert_eq!(high_calls.load(Ordering::Relaxed), 1);
        assert_eq!(unsupported_calls.load(Ordering::Relaxed), 1);
        assert_eq!(low_calls.load(Ordering::Relaxed), 1);

        let cpu = auto.clone().with_device(DevicePreference::Cpu);
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
        let mesh = admissible_cube();
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
