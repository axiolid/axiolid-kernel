//! Stable, transport-independent capability identifiers.

use core::fmt;

/// Versioned semantic operation identity, independent of providers and wire encoding.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(&'static str);

impl CapabilityId {
    pub const fn from_static(value: &'static str) -> Self {
        Self(value)
    }
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Debug for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CapabilityId").field(&self.0).finish()
    }
}
impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

pub mod capability_ids {
    use super::CapabilityId;
    pub const TESSELLATE: CapabilityId =
        CapabilityId::from_static("org.axiolid.geometry.tessellate.v1");
    pub const MESH_BOOLEAN: CapabilityId =
        CapabilityId::from_static("org.axiolid.geometry.mesh-boolean.v1");
    pub const MESH_SECTION: CapabilityId =
        CapabilityId::from_static("org.axiolid.geometry.mesh-section.v1");
    pub const GRAPH_TO_MESH: CapabilityId =
        CapabilityId::from_static("org.axiolid.geometry.graph-to-mesh.v1");
    pub const ALL: [CapabilityId; 4] = [TESSELLATE, MESH_BOOLEAN, MESH_SECTION, GRAPH_TO_MESH];
}

#[cfg(test)]
mod tests {
    use super::capability_ids::ALL;
    use std::collections::BTreeSet;

    #[test]
    fn ids_are_unique_versioned_ascii_tokens() {
        let unique: BTreeSet<_> = ALL.iter().map(|id| id.as_str()).collect();
        assert_eq!(unique.len(), ALL.len());
        for id in ALL {
            let text = id.as_str();
            assert!(text.ends_with(".v1"));
            assert!(text
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-')));
        }
    }
}
