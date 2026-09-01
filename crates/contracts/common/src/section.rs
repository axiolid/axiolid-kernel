//! Mesh plane-section capability and executable provider registry.
//!
//! The input mesh is an explicit approximation boundary. This contract does not
//! claim that section contours recover analytic curves that were lost during
//! tessellation. Providers return deterministic plane-local closed polylines and
//! evidence that records the discrete source.

use std::sync::Arc;

use axiolid_core::{Frame3, Point2};
use axiolid_mesh::{audit_mesh_scratch_bytes, try_audit_mesh, TriMesh};

use crate::{
    Backend, BackendId, CancellationGranularity, DevicePreference, ExecutionOptions,
    ExecutionTarget, GeomError, GeomResult, Operation, ScratchRequirement, SolidRequirements,
};

/// Hard bounds for one mesh section request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionLimits {
    /// Maximum source positions inspected.
    pub max_source_vertices: usize,
    /// Maximum source triangles inspected.
    pub max_source_triangles: usize,
    /// Maximum vertices across all output contours.
    pub max_output_vertices: usize,
    /// Maximum output contours.
    pub max_contours: usize,
}

impl SectionLimits {
    /// Construct explicit source and output work limits.
    pub const fn new(
        max_source_vertices: usize,
        max_source_triangles: usize,
        max_output_vertices: usize,
        max_contours: usize,
    ) -> Self {
        Self {
            max_source_vertices,
            max_source_triangles,
            max_output_vertices,
            max_contours,
        }
    }
}

/// One closed plane-local polyline.
///
/// The terminal point is implicit and must not duplicate the first point.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionContour {
    /// Plane-local points in traversal order.
    pub points: Vec<Point2>,
    /// Private invariant marker: every constructible contour is closed.
    closed: bool,
}

impl SectionContour {
    /// Construct a closed contour. Registry validation checks cardinality and
    /// finite, non-duplicated coordinates after provider dispatch.
    pub fn new(points: Vec<Point2>) -> Self {
        Self {
            points,
            closed: true,
        }
    }

    /// Whether this polyline closes from its last point back to its first.
    pub const fn is_closed(&self) -> bool {
        self.closed
    }
}

/// Provenance of a plane-section approximation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SectionSource {
    /// Contours were intersected with the supplied triangle mesh.
    InputMesh,
}

/// Auditable counts for one section operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionEvidence {
    /// Approximation source.
    pub source: SectionSource,
    /// Source triangles inspected.
    pub source_triangles: usize,
    /// Output contour vertices.
    pub output_vertices: usize,
    /// Output contours.
    pub output_contours: usize,
}

impl SectionEvidence {
    /// Record evidence for a section derived from the input mesh.
    pub const fn input_mesh(
        source_triangles: usize,
        output_vertices: usize,
        output_contours: usize,
    ) -> Self {
        Self {
            source: SectionSource::InputMesh,
            source_triangles,
            output_vertices,
            output_contours,
        }
    }

    /// Whether this result came from the discrete input mesh.
    pub const fn is_derived_from_input_mesh(self) -> bool {
        matches!(self.source, SectionSource::InputMesh)
    }
}

/// Plane-local closed section contours plus approximation evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionOutcome {
    /// Right-handed orthonormal mapping from local `(x,y,0)` to model space.
    pub frame: Frame3,
    /// Closed section contours. Empty means the plane misses the solid.
    pub contours: Vec<SectionContour>,
    /// Source and output counts.
    pub evidence: SectionEvidence,
}

impl SectionOutcome {
    /// Construct a provider result. Registries validate it before returning it.
    pub fn new(frame: Frame3, contours: Vec<SectionContour>, evidence: SectionEvidence) -> Self {
        Self {
            frame,
            contours,
            evidence,
        }
    }
}

/// Provider for deterministic sections of closed oriented triangle solids.
pub trait MeshPlaneSection: Backend {
    /// Scratch needed beyond inputs and result.
    fn scratch_requirement(&self) -> ScratchRequirement {
        ScratchRequirement::Unbounded
    }

    /// How finely the provider polls cancellation.
    fn cancellation_granularity(&self) -> CancellationGranularity {
        CancellationGranularity::None
    }

    /// Intersect one validated closed oriented mesh with `frame`'s local XY
    /// plane. Implementations must enforce `limits` before growing output.
    fn section(
        &self,
        mesh: &TriMesh,
        frame: Frame3,
        limits: SectionLimits,
        options: &ExecutionOptions,
    ) -> GeomResult<SectionOutcome>;
}

#[derive(Debug, Clone)]
struct RegisteredSection {
    priority: i32,
    provider: Arc<dyn MeshPlaneSection>,
}

/// Ordered executable mesh plane-section providers.
///
/// Only unsupported/unavailable providers permit fallback. Geometry,
/// degeneracy, budget, cancellation, and contract errors fail immediately.
#[derive(Debug, Clone, Default)]
pub struct MeshPlaneSectionRegistry {
    providers: Vec<RegisteredSection>,
}

impl MeshPlaneSectionRegistry {
    /// Empty registry.
    pub const fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register an implementation. Higher priorities run first.
    pub fn register<P>(&mut self, priority: i32, provider: P)
    where
        P: MeshPlaneSection + 'static,
    {
        self.register_arc(priority, Arc::new(provider));
    }

    /// Register a shared implementation. Higher priorities run first.
    pub fn register_arc(&mut self, priority: i32, provider: Arc<dyn MeshPlaneSection>) {
        self.providers
            .push(RegisteredSection { priority, provider });
        self.providers
            .sort_by_key(|entry| std::cmp::Reverse(entry.priority));
    }

    /// Registered providers in dispatch order.
    pub fn providers(&self) -> impl Iterator<Item = &dyn MeshPlaneSection> {
        self.providers.iter().map(|entry| entry.provider.as_ref())
    }

    /// Validate and execute one plane-section request.
    pub fn section(
        &self,
        mesh: &TriMesh,
        frame: Frame3,
        limits: SectionLimits,
        options: &ExecutionOptions,
    ) -> GeomResult<SectionOutcome> {
        options.check_cancelled()?;
        validate_source_shape(mesh, frame, limits, options)?;

        let mut has_matching_provider = false;
        let mut has_budgeted_provider = false;
        for entry in &self.providers {
            let descriptor = entry.provider.descriptor();
            if matches_device(options.device(), descriptor.id, descriptor.target) {
                has_matching_provider = true;
                has_budgeted_provider |=
                    section_scratch_fits(entry.provider.scratch_requirement(), mesh, options);
            }
        }
        if !has_matching_provider {
            return Err(GeomError::Unsupported {
                backend: BackendId::new("mesh-section-registry"),
                operation: Operation::MeshPlaneSection,
            });
        }
        if !has_budgeted_provider {
            return Err(GeomError::BudgetExceeded { resource: "memory" });
        }

        // The allocating topology audit happens only after at least one provider
        // and the audit itself have been admitted by the memory budget.
        validate_source_topology(mesh, options)?;

        let mut last_retryable = None;
        for entry in &self.providers {
            let descriptor = entry.provider.descriptor();
            if !matches_device(options.device(), descriptor.id, descriptor.target)
                || !section_scratch_fits(entry.provider.scratch_requirement(), mesh, options)
            {
                continue;
            }
            match entry.provider.section(mesh, frame, limits, options) {
                Ok(outcome) => {
                    validate_outcome(descriptor.id, mesh, frame, limits, &outcome)?;
                    return Ok(outcome);
                }
                Err(error @ (GeomError::Unsupported { .. } | GeomError::Unavailable { .. })) => {
                    last_retryable = Some(error);
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_retryable.unwrap_or(GeomError::Unsupported {
            backend: BackendId::new("mesh-section-registry"),
            operation: Operation::MeshPlaneSection,
        }))
    }
}

fn validate_source_shape(
    mesh: &TriMesh,
    frame: Frame3,
    limits: SectionLimits,
    options: &ExecutionOptions,
) -> GeomResult<()> {
    if mesh.positions.len() > limits.max_source_vertices {
        return Err(GeomError::BudgetExceeded {
            resource: "section source vertices",
        });
    }
    if mesh.triangle_count() > limits.max_source_triangles {
        return Err(GeomError::BudgetExceeded {
            resource: "section source triangles",
        });
    }
    if mesh.indices.is_empty() {
        return Err(GeomError::InvalidInput(
            "section source mesh has no triangles".into(),
        ));
    }
    validate_frame(frame, options.tolerance().angular())?;

    Ok(())
}

fn validate_source_topology(mesh: &TriMesh, options: &ExecutionOptions) -> GeomResult<()> {
    let health = try_audit_mesh(mesh, options.tolerance())
        .map_err(|_| GeomError::BudgetExceeded { resource: "memory" })?;
    if !health.is_closed_two_manifold() {
        return Err(GeomError::NotManifold(format!(
            "plane section requires a closed consistently wound two-manifold mesh; \
             boundary={}, non_manifold={}, inconsistent_winding={}, degenerate={}",
            health.boundary_edges,
            health.non_manifold_edges,
            health.inconsistent_winding_edges,
            health.degenerate_triangles
        )));
    }
    SolidRequirements::Oriented.validate(mesh, "section source")?;
    Ok(())
}

fn section_scratch_fits(
    provider: ScratchRequirement,
    mesh: &TriMesh,
    options: &ExecutionOptions,
) -> bool {
    let Some(budget) = options.memory_budget_bytes() else {
        return true;
    };
    let Some(audit_bytes) = audit_mesh_scratch_bytes(mesh.triangle_count()) else {
        return false;
    };
    let elements = mesh.positions.len().max(mesh.triangle_count());
    let Some(provider_bytes) = provider.upper_bound_bytes(elements) else {
        return false;
    };
    // Audit and provider phases are sequential, so their peak is the maximum,
    // not their sum. Output storage is outside the scratch budget by contract.
    audit_bytes.max(provider_bytes) <= budget
}

fn validate_frame(frame: Frame3, angular_tolerance: f64) -> GeomResult<()> {
    let values = [frame.origin, frame.x, frame.y, frame.z];
    if !values.iter().all(|value| value.is_finite()) {
        return Err(GeomError::InvalidInput(
            "section frame components must be finite".into(),
        ));
    }
    // Structural frame validity cannot be relaxed into zero/scaled axes by a
    // coarse geometric tolerance. Tighter caller tolerances still apply.
    let limit = angular_tolerance.clamp(128.0 * f64::EPSILON, 1.0e-6);
    for (name, axis) in [("x", frame.x), ("y", frame.y), ("z", frame.z)] {
        if (axis.length() - 1.0).abs() > limit {
            return Err(GeomError::InvalidInput(format!(
                "section frame {name} axis must be unit length"
            )));
        }
    }
    if frame.x.dot(frame.y).abs() > limit
        || frame.x.dot(frame.z).abs() > limit
        || frame.y.dot(frame.z).abs() > limit
        || (frame.x.cross(frame.y).dot(frame.z) - 1.0).abs() > limit
    {
        return Err(GeomError::InvalidInput(
            "section frame must be right-handed and orthonormal".into(),
        ));
    }
    Ok(())
}

fn validate_outcome(
    backend: BackendId,
    mesh: &TriMesh,
    frame: Frame3,
    limits: SectionLimits,
    outcome: &SectionOutcome,
) -> GeomResult<()> {
    let violation = |detail: &str| GeomError::BackendContractViolation {
        backend,
        detail: detail.into(),
    };
    if outcome.frame != frame {
        return Err(violation("provider changed the requested section frame"));
    }
    if outcome.contours.len() > limits.max_contours {
        return Err(GeomError::BudgetExceeded {
            resource: "section contours",
        });
    }
    let mut vertices = 0usize;
    for contour in &outcome.contours {
        if contour.points.len() < 3 {
            return Err(violation(
                "section contour must be closed with at least three points",
            ));
        }
        if !contour.points.iter().all(|point| point.is_finite()) {
            return Err(violation("section contour contains a non-finite point"));
        }
        for index in 0..contour.points.len() {
            if contour.points[index] == contour.points[(index + 1) % contour.points.len()] {
                return Err(violation(
                    "section contour contains adjacent duplicate points",
                ));
            }
        }
        vertices = vertices
            .checked_add(contour.points.len())
            .ok_or(GeomError::BudgetExceeded {
                resource: "section output vertices",
            })?;
        if vertices > limits.max_output_vertices {
            return Err(GeomError::BudgetExceeded {
                resource: "section output vertices",
            });
        }
    }
    if outcome.evidence.source_triangles != mesh.triangle_count()
        || outcome.evidence.output_vertices != vertices
        || outcome.evidence.output_contours != outcome.contours.len()
        || !outcome.evidence.is_derived_from_input_mesh()
    {
        return Err(violation(
            "section evidence does not match input/output counts",
        ));
    }
    Ok(())
}

fn matches_device(preference: DevicePreference, id: BackendId, target: ExecutionTarget) -> bool {
    match preference {
        DevicePreference::Auto => true,
        DevicePreference::Cpu => matches!(
            target,
            ExecutionTarget::PortableCpu | ExecutionTarget::OptimizedCpu
        ),
        DevicePreference::Gpu => matches!(target, ExecutionTarget::Gpu),
        DevicePreference::Backend(required) => required == id,
    }
}
