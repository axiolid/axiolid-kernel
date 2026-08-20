//! Axis-aligned bounds used by broad-phase algorithms.

use crate::{Point3, Scalar, Vec3};

/// Axis-aligned three-dimensional bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// Minimum corner.
    pub min: Point3,
    /// Maximum corner.
    pub max: Point3,
}

impl Aabb {
    /// An empty box that absorbs the first extended point.
    pub const fn empty() -> Self {
        Self {
            min: Vec3::splat(Scalar::INFINITY),
            max: Vec3::splat(Scalar::NEG_INFINITY),
        }
    }

    /// Grow to include a point.
    #[inline]
    pub fn extend(&mut self, point: Point3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// Whether the boxes overlap. Touching counts as overlap.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Whether no point has been added.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x
    }

    /// Diagonal vector, or zero for an empty box.
    pub fn diagonal(&self) -> Vec3 {
        if self.is_empty() {
            Vec3::ZERO
        } else {
            self.max - self.min
        }
    }
}

impl Default for Aabb {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_box_absorbs_first_point() {
        let mut bounds = Aabb::default();
        assert!(bounds.is_empty());
        bounds.extend(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(bounds.min, bounds.max);
    }

    #[test]
    fn disjoint_boxes_do_not_intersect() {
        let mut left = Aabb::default();
        left.extend(Vec3::ZERO);
        left.extend(Vec3::ONE);
        let mut right = Aabb::default();
        right.extend(Vec3::splat(2.0));
        right.extend(Vec3::splat(3.0));
        assert!(!left.intersects(&right));
    }
}
