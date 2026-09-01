//! Compile-time API ergonomics expected by downstream Rust clients.

use std::fmt::Debug;
#[cfg(any(feature = "model", feature = "topology"))]
use std::fmt::Display;
#[cfg(any(feature = "model", feature = "topology"))]
use std::hash::Hash;

fn value<T: Debug + Clone + PartialEq>() {}
fn default_value<T: Debug + Default>() {}
#[cfg(any(feature = "model", feature = "topology"))]
fn id<T: Debug + Display + Copy + Eq + Ord + Hash>() {}
fn error<T: std::error::Error + Send + Sync + 'static>() {}

#[test]
fn default_surface_has_standard_traits() {
    value::<axiolid::Tolerance>();
    value::<axiolid::Aabb>();
    value::<axiolid::mesh::TriMesh>();
    default_value::<axiolid::Aabb>();
    default_value::<axiolid::mesh::TriMesh>();
    error::<axiolid_core::ToleranceError>();
    error::<axiolid::mesh::MeshValidationError>();
}

#[test]
fn default_execution_errors_are_standard_errors() {
    error::<axiolid_backend_cpu::CpuConfigError>();
}

#[cfg(feature = "contracts")]
#[test]
fn kernel_errors_are_standard_errors() {
    error::<axiolid::contracts::GeomError>();
}

#[cfg(feature = "model")]
#[test]
fn model_handles_and_values_have_standard_traits() {
    id::<axiolid::model::NodeId>();
    value::<axiolid::model::GeometryGraph>();
    value::<axiolid::model::GeometryNode>();
    value::<axiolid::model::OpenProfile>();
    default_value::<axiolid::model::GeometryGraph>();
    default_value::<axiolid::model::GeometryGraphBuilder>();
}

#[cfg(feature = "profiles")]
#[test]
fn profile_values_are_debuggable_and_cloneable() {
    value::<axiolid::profile::Profile>();
}

#[cfg(feature = "curves")]
#[test]
fn curve_values_are_debuggable_and_cloneable() {
    value::<axiolid::curve::Curve2>();
    value::<axiolid::curve::Curve3>();
}

#[cfg(feature = "surfaces")]
#[test]
fn surface_values_are_debuggable_and_cloneable() {
    value::<axiolid::surface::Surface>();
}

#[cfg(feature = "topology")]
#[test]
fn topology_handles_are_typed_value_ids() {
    id::<axiolid::topology::VertexId>();
    id::<axiolid::topology::EdgeId>();
    id::<axiolid::topology::FaceId>();
}
