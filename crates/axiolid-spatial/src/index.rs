//! Zero-allocation spatial index query contract.

use core::ops::ControlFlow;

use axiolid_core::{Aabb, Ray3, Scalar};

/// Key and bounds supplied to an index builder.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialItem<K> {
    /// Caller-owned identity.
    pub key: K,
    /// Broad-phase bounds.
    pub bounds: Aabb,
}

/// Ray hit returned in ascending distance order where supported.
#[derive(Debug, Clone, PartialEq)]
pub struct RayHit<K> {
    /// Caller key.
    pub key: K,
    /// Nonnegative ray parameter.
    pub distance: Scalar,
}

/// Read-only broad-phase query API. Callback visitation avoids allocating a
/// result vector for large clash batches.
pub trait SpatialIndex<K>: core::fmt::Debug + Send + Sync {
    /// Visit keys whose stored bounds overlap `query`. Returning `Break` stops.
    fn visit_aabb(&self, query: &Aabb, visitor: &mut dyn FnMut(&K) -> ControlFlow<()>);

    /// Visit broad-phase ray candidates.
    fn visit_ray(&self, ray: &Ray3, visitor: &mut dyn FnMut(RayHit<&K>) -> ControlFlow<()>);

    /// Number of indexed items.
    fn len(&self) -> usize;

    /// Whether no items are indexed.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
