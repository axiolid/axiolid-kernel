#![forbid(unsafe_code)]

//! Metric-property contracts.
//!
//! Algorithms are generic over representation and return structured failures;
//! an open shell must not silently report a plausible volume.

pub mod measure;
pub mod mesh;
pub mod properties;
pub mod proximity;
pub mod winding;

pub use measure::Measure;
pub use mesh::{
    surface_properties, volume_properties, MeshMeasureError, SurfaceProperties, VolumeProperties,
};
pub use properties::MassProperties;
pub use proximity::{
    closest_point_on_triangle, closest_points_on_segments, closest_points_on_triangles,
    ClosestPoints3, ProximityError,
};
pub use winding::{WindingError, WindingMesh, WindingNumber};
