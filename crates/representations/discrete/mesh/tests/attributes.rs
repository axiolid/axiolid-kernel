//! Attribute channels: structural validation and honest fate reporting.

use axiolid_core::Point3;
use axiolid_mesh::{AttributeChannel, Blend, MeshValidationError, TriMesh};

fn tri() -> TriMesh {
    TriMesh::new(
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        vec![0, 1, 2],
    )
}

/// A channel must carry one tuple per vertex.
///
/// The failure this prevents is silent misalignment: a channel one vertex
/// short still indexes fine for every earlier vertex, so the corruption
/// only shows up at the end of the buffer, far from its cause.
#[test]
fn a_channel_must_cover_every_vertex() {
    let mut mesh = tri();
    mesh.attributes.push(AttributeChannel::new(
        "uv",
        // Two vertices' worth of UV for a three-vertex mesh.
        vec![0.0, 0.0, 1.0, 0.0],
        2,
        Blend::Linear,
    ));

    assert_eq!(
        mesh.validate_structure(),
        Err(MeshValidationError::AttributeCount {
            name: "uv".to_owned(),
            expected: 3,
            actual: 2,
        })
    );
}

/// A well-formed channel passes.
#[test]
fn a_matching_channel_validates() {
    let mut mesh = tri();
    mesh.attributes.push(AttributeChannel::new(
        "uv",
        vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
        2,
        Blend::Linear,
    ));
    assert_eq!(mesh.validate_structure(), Ok(()));
}

/// Two channels cannot share a name.
///
/// A duplicate makes lookup ambiguous, and the ambiguity would be resolved
/// silently by whichever happened to come first.
#[test]
fn duplicate_channel_names_are_refused() {
    let mut mesh = tri();
    for _ in 0..2 {
        mesh.attributes.push(AttributeChannel::new(
            "id",
            vec![1.0, 2.0, 3.0],
            1,
            Blend::Nearest,
        ));
    }
    assert_eq!(
        mesh.validate_structure(),
        Err(MeshValidationError::AttributeDuplicateName {
            name: "id".to_owned(),
        })
    );
}

/// A zero width covers no vertices and is refused by name.
///
/// Reported as its own error rather than as a count mismatch: dividing by
/// the width to compare counts would hide the real defect.
#[test]
fn a_zero_width_channel_is_refused() {
    let mut mesh = tri();
    mesh.attributes
        .push(AttributeChannel::new("empty", Vec::new(), 0, Blend::None));
    assert_eq!(
        mesh.validate_structure(),
        Err(MeshValidationError::AttributeZeroWidth {
            name: "empty".to_owned(),
        })
    );
}

/// Tuple access is bounds-checked rather than panicking.
#[test]
fn reading_a_tuple_is_bounds_checked() {
    let channel = AttributeChannel::new("uv", vec![0.0, 0.0, 1.0, 0.5], 2, Blend::Linear);
    assert_eq!(channel.vertex_count(), 2);
    assert_eq!(channel.get(1), Some(&[1.0, 0.5][..]));
    assert_eq!(channel.get(2), None, "past the end is None, not a panic");
}
