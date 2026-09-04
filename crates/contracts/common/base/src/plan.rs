//! Reproducible operation plans.
//!
//! # Why a plan and not just options
//!
//! [`ExecutionOptions`] says how to run one operation. It does not say what
//! was run, what it produced, or whether re-running it would produce the same
//! thing. A graph evaluated twice under the same options gave no guarantee of
//! the same result, and nothing recorded which inputs and budgets produced a
//! given output.
//!
//! A [`Plan`] is that missing artifact: the options, plus the recorded
//! provenance of every step, plus the outcome. Re-executing a plan against the
//! same inputs must produce the same result, and the plan itself is the
//! evidence of what was run.
//!
//! # Determinism is requested, not assumed
//!
//! [`Determinism`] has always been declarable, but nothing read it: a caller
//! could ask for [`Determinism::Bitwise`] and receive best-effort output with
//! no indication the request was ignored. A plan closes that hole. Executing a
//! plan admits the requested level against what the executing provider
//! actually guarantees, and refuses when the request cannot be met.
//!
//! Refusing is the point. Silently accepting a determinism request a provider
//! cannot honour is exactly the class of quiet wrongness this kernel exists to
//! avoid: the caller believes it can hash the result and compare it across
//! machines, and it cannot.
//!
//! # Scope
//!
//! A plan is an in-process artifact. It is deliberately not serialised: a wire
//! format is a compatibility promise, and freezing one before the public API
//! stabilises would commit to a shape the kernel has not finished learning.
//! Reproducibility is proved by re-executing the same plan, not by persisting
//! it. Serialisation can be added without changing this contract.

use crate::capability::{BackendId, Operation};
use crate::error::{GeomError, GeomResult};
use crate::execution::{Determinism, ExecutionOptions, ScratchRequirement};

/// One recorded step: what ran, where, and under what guarantee.
///
/// Provenance survives across operations by accumulating these in order. A
/// step records the guarantee the provider actually delivered, not the one the
/// caller asked for, so a plan that ran at a weaker level than requested is
/// visible after the fact rather than indistinguishable from one that did not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStep {
    /// Operation performed.
    pub operation: Operation,
    /// Backend that performed it.
    pub backend: BackendId,
    /// Determinism the backend actually guaranteed for this step.
    pub guaranteed: Determinism,
    /// Stable description of the input family this step consumed.
    pub input: String,
}

impl PlanStep {
    /// Record a step.
    pub fn new(
        operation: Operation,
        backend: BackendId,
        guaranteed: Determinism,
        input: &str,
    ) -> Self {
        Self {
            operation,
            backend,
            guaranteed,
            input: input.to_owned(),
        }
    }
}

/// A reproducible operation plan.
///
/// Carries the options every step runs under and the provenance of the steps
/// taken so far. Re-executing the same plan against the same inputs must
/// produce the same result; the recorded steps are the evidence of what ran.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    options: ExecutionOptions,
    steps: Vec<PlanStep>,
}

impl Plan {
    /// Start a plan from the options every step will run under.
    #[must_use]
    pub fn new(options: ExecutionOptions) -> Self {
        Self {
            options,
            steps: Vec::new(),
        }
    }

    /// Options every step of this plan runs under.
    #[must_use]
    pub fn options(&self) -> &ExecutionOptions {
        &self.options
    }

    /// Steps recorded so far, in execution order.
    #[must_use]
    pub fn steps(&self) -> &[PlanStep] {
        &self.steps
    }
}

impl Plan {
    /// Admit and record a step, or refuse it.
    ///
    /// Two admissions, both fail-closed:
    ///
    /// - The provider's `guaranteed` determinism must be at least the level
    ///   the plan requested. A provider that cannot meet the request is
    ///   refused rather than silently downgraded, because the caller has no
    ///   other way to learn the guarantee it relied on was not delivered.
    /// - The step's scratch requirement must fit the plan's memory budget.
    ///   Exhaustion is a typed `BudgetExceeded`, never a silent truncation to
    ///   a smaller result that still looks plausible.
    pub fn admit(
        &mut self,
        step: PlanStep,
        scratch: ScratchRequirement,
        elements: usize,
    ) -> GeomResult<()> {
        if !step.guaranteed.satisfies(self.options.determinism()) {
            return Err(GeomError::BackendContractViolation {
                backend: step.backend,
                detail: format!(
                    "plan requires {:?} determinism, backend guarantees only {:?}",
                    self.options.determinism(),
                    step.guaranteed
                ),
            });
        }
        if !scratch.fits_budget(&self.options, elements) {
            return Err(GeomError::BudgetExceeded {
                resource: "plan step scratch memory",
            });
        }
        self.steps.push(step);
        Ok(())
    }
}
