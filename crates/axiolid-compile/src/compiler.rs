//! The `GeometryCompiler` implementation: graph in, meshes out.
//!
//! Evaluation is iterative. The graph forbids non-prior references,
//! so cycles are structurally impossible, but depth is unbounded and
//! recursion would risk a stack overflow on adversarial input.

use axiolid_core::{Point3, Scalar, Tolerance, Transform3};
use axiolid_kernel::{
    Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomError,
    GeomResult, GeometryCompiler, MeshBoolean, Operation, ScratchRequirement,
};
use axiolid_mesh::TriMesh;
use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};

use axiolid_generate::extrude::extrude_profile;
use axiolid_generate::profile::profile_rings;

/// Scalar reference compiler.
///
/// Generic over the boolean provider so this crate never depends on a
/// particular one: `axiolid-boolmesh` is an adapter, and a different provider
/// swaps in without touching this code.
#[derive(Debug, Clone)]
pub struct ScalarCompiler<B> {
    boolean: B,
}

impl<B> ScalarCompiler<B> {
    /// Bind a boolean provider.
    pub const fn new(boolean: B) -> Self {
        Self { boolean }
    }

    /// The bound provider.
    pub const fn boolean(&self) -> &B {
        &self.boolean
    }
}

impl<B: MeshBoolean> Backend for ScalarCompiler<B> {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(
            BackendId::new("scalar-compile"),
            ExecutionTarget::PortableCpu,
        )
    }
}

/// One entry in the explicit evaluation stack.
///
/// `Enter` schedules dependency discovery; `Exit` runs after every dependency
/// already has a mesh, which is what replaces the recursive call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EvalKey {
    id: NodeId,
    linear_bits: u64,
    angular_bits: u64,
}

impl EvalKey {
    fn new(id: NodeId, tolerance: Tolerance) -> Self {
        let bits = |value: Scalar| if value == 0.0 { 0 } else { value.to_bits() };
        Self {
            id,
            linear_bits: bits(tolerance.linear()),
            angular_bits: bits(tolerance.angular()),
        }
    }

    fn tolerance(self) -> Tolerance {
        Tolerance::new(
            Scalar::from_bits(self.linear_bits),
            Scalar::from_bits(self.angular_bits),
        )
        .expect("evaluation keys only contain validated tolerances")
    }
}

#[derive(Debug, Clone, Copy)]
enum Step {
    Enter(EvalKey),
    Exit(EvalKey),
}

/// Cached meshes keyed by node and effective local tolerance.
///
/// A transformed instance changes the local chord budget. Keying only by node
/// would incorrectly reuse a coarse source mesh for a larger instance.
type Cache = std::collections::HashMap<EvalKey, TriMesh>;

impl<B: MeshBoolean> ScalarCompiler<B> {
    /// Resolve a node handle, blaming the graph rather than panicking.
    /// Resolve a closed 2D boundary curve into rings.
    ///
    /// Only a closed polyline boundary is handled: its points ARE the ring.
    /// An analytic boundary needs the curve evaluator, and an open one does
    /// not bound anything, so both are refused rather than guessed at.
    fn boundary_rings(
        &self,
        graph: &GeometryGraph,
        id: NodeId,
    ) -> GeomResult<axiolid_generate::profile::Rings> {
        let node = self.node(graph, id)?;
        let GeometryNode::Curve2(curve) = node else {
            return Err(GeomError::InvalidInput(format!(
                "half-space boundary {id:?} is not a Curve2 node"
            )));
        };
        match curve {
            axiolid_curve::Curve2::Polyline(p) => {
                let mut pts = p.points.clone();
                // A closed polyline may or may not repeat its first point.
                // Dropping the duplicate keeps the ring's edge count honest.
                if pts.len() >= 2 && pts[0] == pts[pts.len() - 1] {
                    pts.pop();
                }
                if pts.len() < 3 {
                    return Err(GeomError::InvalidInput(
                        "half-space boundary needs at least 3 distinct points".to_owned(),
                    ));
                }
                Ok(axiolid_generate::profile::Rings {
                    outer: pts,
                    holes: Vec::new(),
                })
            }
            _ => Err(GeomError::Unsupported {
                backend: self.descriptor().id,
                operation: Operation::CurveEvaluation,
            }),
        }
    }

    /// Resolve a node that must be a profile, into flattened rings.
    /// Resolve a node that must be a surface.
    ///
    /// Reported by node id rather than by position so a malformed graph
    /// names the offending node instead of the operation that reached it.
    fn surface_of(
        graph: &GeometryGraph,
        id: axiolid_model::NodeId,
    ) -> GeomResult<&axiolid_surface::Surface> {
        match graph.get(id) {
            Some(GeometryNode::Surface(surface)) => Ok(surface),
            Some(_) => Err(GeomError::InvalidInput(format!(
                "reference surface {id:?} is not a Surface node"
            ))),
            None => Err(GeomError::InvalidInput(format!(
                "reference surface {id:?} does not belong to this graph"
            ))),
        }
    }

    fn surface_normals(
        &self,
        graph: &GeometryGraph,
        id: NodeId,
        path: &[Point3],
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<axiolid_core::Vec3>> {
        if let Some(GeometryNode::SurfaceRelation(
            axiolid_model::SurfaceRelation::LinearExtrusion { direction, .. },
        )) = graph.get(id)
        {
            return axiolid_generate::sweep::linear_extrusion_normals(path, *direction);
        }
        let surface = Self::surface_of(graph, id)?;
        path.iter()
            .map(|point| {
                let (u, v) = axiolid_scalar::surface::invert(surface, *point, options.tolerance())?;
                axiolid_scalar::surface::normal(surface, u, v)
            })
            .collect()
    }

    fn rings_of(
        &self,
        graph: &GeometryGraph,
        id: NodeId,
        options: &ExecutionOptions,
        what: &str,
    ) -> GeomResult<axiolid_generate::profile::Rings> {
        let node = self.node(graph, id)?;
        let GeometryNode::Profile(shape) = node else {
            return Err(GeomError::InvalidInput(format!(
                "{what} {id:?} is not a Profile node"
            )));
        };
        profile_rings(shape, chord_error(options), options.tolerance())
    }

    /// Sample a directrix curve into a polyline.
    ///
    /// Only a polyline directrix is handled here: its points ARE the samples,
    /// so no evaluator is involved. An analytic directrix needs the curve
    /// provider, which the compiler does not yet hold, and is refused with the
    /// named capability rather than silently approximated by its control
    /// points.
    fn directrix_points(
        &self,
        graph: &GeometryGraph,
        id: NodeId,
        range: Option<(Scalar, Scalar)>,
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<Point3>> {
        crate::directrix::points(graph, id, range, options)
    }

    fn node<'g>(&self, graph: &'g GeometryGraph, id: NodeId) -> GeomResult<&'g GeometryNode> {
        graph.get(id).ok_or_else(|| {
            GeomError::InvalidInput(format!("node {id:?} does not belong to this graph"))
        })
    }

    /// Nodes that must have meshes before `id` can be evaluated.
    ///
    /// Only mesh-producing dependencies are listed. A profile referenced by an
    /// extrusion is consumed as 2D data, not as a mesh, so it is deliberately
    /// absent: compiling it standalone would be meaningless.
    fn mesh_dependencies(
        &self,
        graph: &GeometryGraph,
        node: &GeometryNode,
        key: EvalKey,
    ) -> GeomResult<Vec<EvalKey>> {
        let tolerance = key.tolerance();
        let same = |id| EvalKey::new(id, tolerance);
        Ok(match node {
            GeometryNode::Instance(instance) => vec![EvalKey::new(
                instance.source,
                instance_local_tolerance(instance.transform, tolerance)?,
            )],
            GeometryNode::Collection(members) => members.iter().copied().map(same).collect(),
            GeometryNode::SolidOperation(
                operation @ SolidOperation::Boolean { left, right, .. },
            ) => {
                if is_subject_bounded_half_space_boolean(graph, operation) {
                    vec![same(*left)]
                } else {
                    vec![same(*left), same(*right)]
                }
            }
            _ => Vec::new(),
        })
    }

    /// Iterative post-order evaluation of one root.
    fn evaluate(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
        cache: &mut Cache,
    ) -> GeomResult<TriMesh> {
        let root_key = EvalKey::new(root, options.tolerance());
        let mut stack = vec![Step::Enter(root_key)];
        while let Some(step) = stack.pop() {
            match step {
                Step::Enter(key) => {
                    if cache.contains_key(&key) {
                        continue;
                    }
                    let node = self.node(graph, key.id)?;
                    let deps = self.mesh_dependencies(graph, node, key)?;
                    // Exit runs after every dependency, so push it first.
                    stack.push(Step::Exit(key));
                    for dep in deps {
                        if !cache.contains_key(&dep) {
                            stack.push(Step::Enter(dep));
                        }
                    }
                }
                Step::Exit(key) => {
                    if cache.contains_key(&key) {
                        continue;
                    }
                    let local_options = options.clone().with_tolerance(key.tolerance());
                    let mesh = self.build(graph, key, &local_options, cache)?;
                    cache.insert(key, mesh);
                }
            }
        }
        cache
            .get(&root_key)
            .cloned()
            .ok_or_else(|| GeomError::InvalidInput(format!("root {root:?} produced no mesh")))
    }

    /// Convert authored triangular faces by preserving their exact corner order.
    fn compile_authored_triangles(&self, mesh: &axiolid_mesh::PolygonMesh) -> GeomResult<TriMesh> {
        if mesh
            .faces
            .iter()
            .any(|face| face.outer.len() != 3 || !face.holes.is_empty())
        {
            return Err(GeomError::Unsupported {
                backend: self.descriptor().id,
                operation: Operation::Tessellation,
            });
        }

        let position_count = mesh.positions.len();
        if let Some(index) = mesh
            .faces
            .iter()
            .flat_map(|face| face.outer.iter().copied())
            .find(|&index| index as usize >= position_count)
        {
            return Err(GeomError::InvalidInput(format!(
                "authored triangle index {index} exceeds position count {position_count}"
            )));
        }

        let mut positions = Vec::new();
        positions
            .try_reserve_exact(position_count)
            .map_err(|_| GeomError::BudgetExceeded {
                resource: "authored triangle positions",
            })?;
        positions.extend_from_slice(&mesh.positions);

        let index_count = mesh
            .faces
            .len()
            .checked_mul(3)
            .ok_or(GeomError::BudgetExceeded {
                resource: "authored triangle indices",
            })?;
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(index_count)
            .map_err(|_| GeomError::BudgetExceeded {
                resource: "authored triangle indices",
            })?;
        for face in &mesh.faces {
            indices.extend_from_slice(&face.outer);
        }

        let triangles = TriMesh::new(positions, indices);
        triangles.validate_structure().map_err(|error| {
            GeomError::InvalidInput(format!("invalid authored triangle mesh: {error}"))
        })?;
        Ok(triangles)
    }

    /// Build one node, assuming its mesh dependencies are already cached.
    fn build(
        &self,
        graph: &GeometryGraph,
        key: EvalKey,
        options: &ExecutionOptions,
        cache: &Cache,
    ) -> GeomResult<TriMesh> {
        let id = key.id;
        let node = self.node(graph, id)?;
        match node {
            GeometryNode::TriMesh(mesh) => Ok(mesh.clone()),
            GeometryNode::PolygonMesh(mesh) => self.compile_authored_triangles(mesh),
            GeometryNode::Instance(instance) => {
                let source_tolerance =
                    instance_local_tolerance(instance.transform, options.tolerance())?;
                let source = self.cached(cache, instance.source, source_tolerance)?;
                Ok(transform_mesh(source, instance.transform))
            }
            GeometryNode::Collection(members) => {
                let mut merged = TriMesh::default();
                for &member in members {
                    append_mesh(
                        &mut merged,
                        self.cached(cache, member, options.tolerance())?,
                    );
                }
                Ok(merged)
            }
            GeometryNode::SolidOperation(operation) => {
                self.build_solid(graph, operation, options, cache)
            }
            GeometryNode::BRep(brep) => crate::brep::tessellate(brep, graph, options.tolerance()),
            // CSG primitives are analytic solids: no surface evaluation,
            // no trim curves, just a closed mesh at the caller's tolerance.
            GeometryNode::Primitive(primitive) => {
                axiolid_scalar::primitive::tessellate_primitive(primitive, options.tolerance())
            }
            other => Err(GeomError::Unsupported {
                backend: self.descriptor().id,
                operation: unsupported_operation(other),
            }),
        }
    }

    /// Read an already-built dependency.
    fn cached<'c>(
        &self,
        cache: &'c Cache,
        id: NodeId,
        tolerance: Tolerance,
    ) -> GeomResult<&'c TriMesh> {
        cache.get(&EvalKey::new(id, tolerance)).ok_or_else(|| {
            GeomError::InvalidInput(format!(
                "dependency {id:?} was not evaluated first at tolerance {tolerance:?}"
            ))
        })
    }

    /// Extrusion and boolean; every other solid family is explicitly refused.
    fn build_solid(
        &self,
        graph: &GeometryGraph,
        operation: &SolidOperation,
        options: &ExecutionOptions,
        cache: &Cache,
    ) -> GeomResult<TriMesh> {
        match operation {
            SolidOperation::Extrusion {
                profile,
                direction,
                depth,
            } => {
                let node = self.node(graph, *profile)?;
                let GeometryNode::Profile(shape) = node else {
                    return Err(GeomError::InvalidInput(format!(
                        "extrusion profile {profile:?} is not a Profile node"
                    )));
                };
                let rings = profile_rings(shape, chord_error(options), options.tolerance())?;
                extrude_profile(&rings, *direction, *depth, options.tolerance())
            }
            SolidOperation::Revolution {
                profile,
                axis_origin,
                axis_direction,
                angle,
            } => {
                let node = self.node(graph, *profile)?;
                let GeometryNode::Profile(shape) = node else {
                    return Err(GeomError::InvalidInput(format!(
                        "revolution profile {profile:?} is not a Profile node"
                    )));
                };
                let rings = profile_rings(shape, chord_error(options), options.tolerance())?;
                axiolid_generate::revolve::revolve(
                    &rings,
                    *axis_origin,
                    *axis_direction,
                    *angle,
                    options.tolerance(),
                )
            }
            SolidOperation::TaperedExtrusion {
                start_profile,
                end_profile,
                direction,
                depth,
            } => {
                let a = self.rings_of(graph, *start_profile, options, "taper start profile")?;
                let b = self.rings_of(graph, *end_profile, options, "taper end profile")?;
                axiolid_generate::sweep::tapered_extrude(&a, &b, *direction, *depth)
            }
            SolidOperation::TaperedRevolution {
                start_profile,
                end_profile,
                axis_origin,
                axis_direction,
                angle,
            } => {
                let a = self.rings_of(graph, *start_profile, options, "taper start profile")?;
                let b = self.rings_of(graph, *end_profile, options, "taper end profile")?;
                axiolid_generate::sweep::tapered_revolve(
                    &a,
                    &b,
                    *axis_origin,
                    *axis_direction,
                    *angle,
                    options.tolerance(),
                )
            }
            SolidOperation::SweptDisk {
                directrix,
                radius,
                inner_radius,
                parameter_range,
                fillet_radius,
            } => {
                let path = self.directrix_points(graph, *directrix, *parameter_range, options)?;
                axiolid_generate::sweep::swept_disk(
                    &path,
                    *radius,
                    *inner_radius,
                    *fillet_radius,
                    options.tolerance(),
                )
            }
            SolidOperation::FixedReferenceSweep {
                profile,
                directrix,
                reference_direction,
                parameter_range,
            } => {
                let rings = self.rings_of(graph, *profile, options, "sweep profile")?;
                let path = self.directrix_points(graph, *directrix, *parameter_range, options)?;
                axiolid_generate::sweep::fixed_reference_sweep(&rings, &path, *reference_direction)
            }
            SolidOperation::SurfaceCurveSweep {
                profile,
                directrix,
                reference_surface,
                parameter_range,
            } => {
                let rings =
                    self.rings_of(graph, *profile, options, "surface curve sweep profile")?;
                let path = self.directrix_points(graph, *directrix, *parameter_range, options)?;
                let normals = self.surface_normals(graph, *reference_surface, &path, options)?;
                axiolid_generate::sweep::surface_curve_sweep(&rings, &path, &normals)
            }
            SolidOperation::SectionedSpine { spine, sections } => {
                let path = self.directrix_points(graph, *spine, None, options)?;
                if sections.len() != path.len() {
                    return Err(GeomError::InvalidInput(format!(
                        "a sectioned spine needs one section per spine point: {} sections, {} points",
                        sections.len(),
                        path.len()
                    )));
                }
                let mut placed = Vec::with_capacity(sections.len());
                for (section, origin) in sections.iter().zip(&path) {
                    let rings =
                        self.rings_of(graph, section.profile, options, "spine section profile")?;
                    // The section's own placement positions its profile;
                    // the spine point supplies the station origin.
                    let pts = rings
                        .outer
                        .iter()
                        .chain(rings.holes.iter().flatten())
                        .map(|p| {
                            section
                                .placement
                                .transform_point3(Point3::new(p.x, p.y, 0.0))
                                + *origin
                        })
                        .collect();
                    placed.push((rings, pts));
                }
                axiolid_generate::sweep::sectioned_spine(&placed)
            }
            SolidOperation::BoundedHalfSpace {
                half_space,
                boundary,
                placement,
            } => {
                let node = self.node(graph, *half_space)?;
                let GeometryNode::HalfSpace(hs) = node else {
                    return Err(GeomError::InvalidInput(format!(
                        "half-space {half_space:?} is not a HalfSpace node"
                    )));
                };
                let rings = self.boundary_rings(graph, *boundary)?;
                // The declared margin is the contract's own knob for how far
                // an unbounded half-space extends before it can be meshed.
                let margin = axiolid_primitive::ClipMargin::new(2.0)
                    .expect("2.0 is a valid positive clip margin");
                let mesh = axiolid_generate::half_space::bounded_half_space(
                    &rings,
                    hs.boundary,
                    hs.agreement,
                    margin,
                    options.tolerance(),
                )?;
                Ok(transform_mesh(&mesh, *placement))
            }
            SolidOperation::Boolean {
                left,
                right,
                operator,
            } => {
                let subject = self.cached(cache, *left, options.tolerance())?;
                let right_node = self.node(graph, *right)?;
                let bounded_tool = if let GeometryNode::HalfSpace(hs) = right_node {
                    Some(axiolid_generate::half_space::for_subject(
                        subject,
                        *hs,
                        options.tolerance(),
                    )?)
                } else {
                    None
                };
                let tool = if let Some(tool) = bounded_tool.as_ref() {
                    tool
                } else {
                    self.cached(cache, *right, options.tolerance())?
                };
                // The compiler produces a mesh graph, so it takes the mesh and
                // drops the evidence here. A caller wanting boolean diagnostics
                // calls the registry directly; threading evidence through every
                // DAG node would change the compile contract, which is a
                // separate decision from fixing the boolean contract.
                self.boolean
                    .boolean(subject, tool, *operator, options)
                    .map(|outcome| outcome.mesh)
            }
            // Every remaining family is a sweep of some kind (revolution,
            // swept disk, fixed-reference, surface-curve, sectioned spine) or
            // a bounded half-space. Naming the capability lets a caller
            // register a provider for it rather than guess.
            _ => Err(GeomError::Unsupported {
                backend: self.descriptor().id,
                operation: Operation::Sweep,
            }),
        }
    }
}

/// Convert a world-space tolerance into conservative instance-local space.
fn instance_local_tolerance(transform: Transform3, tolerance: Tolerance) -> GeomResult<Tolerance> {
    let m = transform.matrix3;
    let sx = m.x_axis.length();
    let sy = m.y_axis.length();
    let sz = m.z_axis.length();
    let max_scale = sx.max(sy).max(sz);
    if !max_scale.is_finite() || max_scale == 0.0 {
        return Err(GeomError::InvalidInput(
            "instance transform has no finite scale".into(),
        ));
    }
    let eps = 32.0 * f64::EPSILON;
    let orthogonal = m.x_axis.dot(m.y_axis).abs() <= eps * sx * sy
        && m.x_axis.dot(m.z_axis).abs() <= eps * sx * sz
        && m.y_axis.dot(m.z_axis).abs() <= eps * sy * sz;
    let stretch = if orthogonal {
        max_scale * (1.0 + 3.0 * eps)
    } else {
        (sx * sx + sy * sy + sz * sz).sqrt()
    };
    if !stretch.is_finite() {
        return Err(GeomError::InvalidInput(
            "instance transform scale overflow".into(),
        ));
    }
    Tolerance::new(tolerance.linear() / stretch, tolerance.angular())
        .map_err(|error| GeomError::InvalidInput(error.to_string()))
}

fn chord_error(options: &ExecutionOptions) -> Scalar {
    options.tolerance().linear()
}

fn is_subject_bounded_half_space_boolean(
    graph: &GeometryGraph,
    operation: &SolidOperation,
) -> bool {
    let SolidOperation::Boolean {
        right, operator, ..
    } = operation
    else {
        return false;
    };
    // These operators stay within the finite left-hand subject, so a prism
    // covering its bounds is an exact finite stand-in for the half-space.
    // Union and XOR are unbounded and must remain unsupported.
    matches!(
        operator,
        axiolid_core::BooleanOperator::Difference | axiolid_core::BooleanOperator::Intersection
    ) && matches!(graph.get(*right), Some(GeometryNode::HalfSpace(_)))
}

/// Which capability a node family would need.
///
/// Reporting the real missing capability lets a caller register a provider
/// that supplies it, instead of guessing from a generic failure.
fn unsupported_operation(node: &GeometryNode) -> Operation {
    match node {
        GeometryNode::Curve2(_) | GeometryNode::Curve3(_) | GeometryNode::OpenProfile(_) => {
            Operation::CurveEvaluation
        }
        GeometryNode::Surface(_) => Operation::SurfaceEvaluation,
        GeometryNode::Profile(_) => Operation::ProfileTriangulation,
        GeometryNode::BRep(_) | GeometryNode::PolygonMesh(_) => Operation::Tessellation,
        _ => Operation::GraphCompilation,
    }
}

/// Apply an affine transform to every position.
///
/// Normals are dropped rather than transformed: a correct normal transform is
/// the inverse transpose, and silently applying the point transform would
/// produce subtly wrong shading under non-uniform scale.
fn transform_mesh(mesh: &TriMesh, transform: Transform3) -> TriMesh {
    let positions = mesh
        .positions
        .iter()
        .map(|&p| transform.transform_point3(p))
        .collect();
    let mut out = TriMesh::new(positions, mesh.indices.clone());
    if transform.matrix3.determinant() < 0.0 {
        // A mirroring transform reverses orientation; restore outward winding
        // so the result still satisfies the boolean provider's precondition.
        for triangle in out.indices.chunks_exact_mut(3) {
            triangle.swap(1, 2);
        }
    }
    out
}

/// Concatenate `source` into `target`, rebasing indices.
fn append_mesh(target: &mut TriMesh, source: &TriMesh) {
    let offset = target.positions.len() as u32;
    target.positions.extend_from_slice(&source.positions);
    target
        .indices
        .extend(source.indices.iter().map(|&i| i + offset));
}

impl<B: MeshBoolean> GeometryCompiler for ScalarCompiler<B> {
    /// Bounded by the peak mesh size, which is data-dependent, so the honest
    /// answer is unbounded rather than an invented constant.
    fn scratch_requirement(&self) -> ScratchRequirement {
        ScratchRequirement::Unbounded
    }

    fn compile(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let mut cache = Cache::new();
        self.evaluate(graph, root, options, &mut cache)
    }

    /// Overriding the `_into` seam gives both call shapes one shared cache,
    /// so a subtree referenced by several roots is compiled once per batch.
    fn compile_batch_into(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
        destination: &mut Vec<TriMesh>,
    ) -> GeomResult<()> {
        destination.reserve(roots.len());
        let mut cache = Cache::new();
        for &root in roots {
            destination.push(self.evaluate(graph, root, options, &mut cache)?);
        }
        Ok(())
    }
}

impl<B: MeshBoolean> ScalarCompiler<B> {
    /// The boolean provider this compiler dispatches to.
    ///
    /// Exposed so an application can apply its own source-format set
    /// operations with the same provider the compiler uses, rather than
    /// constructing a second one that might differ.
    pub const fn boolean_provider(&self) -> &B {
        &self.boolean
    }
}
