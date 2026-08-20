//! Gates for the `subtract_many` batch override.
//!
//! The override's entire justification is that it is FASTER while producing
//! the SAME result. These tests hold the second half: the grouped path must
//! agree with the sequential one on every layout, including the ones where
//! grouping does and does not apply.

mod support;

use axiolid_core::{BooleanOperator, Tolerance};
use axiolid_kernel::{ExecutionOptions, MeshBoolean};
use axiolid_mesh::TriMesh;

use axiolid_boolmesh::BoolmeshBoolean;
use support::{boxx, volume};

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::MILLIMETRE)
}

/// The pre-override behaviour, kept as the reference implementation.
fn sequential(subject: &TriMesh, tools: &[TriMesh]) -> TriMesh {
    let provider = BoolmeshBoolean::new();
    let mut current = subject.clone();
    for tool in tools {
        current = provider
            .boolean(&current, tool, BooleanOperator::Difference, &options())
            .expect("sequential difference");
    }
    current
}

/// Volumes must agree to a relative tolerance, not bitwise: the two paths sum
/// a differently ordered triangle list, so the last bits legitimately differ.
fn assert_same_volume(left: &TriMesh, right: &TriMesh, what: &str) {
    let (a, b) = (volume(left), volume(right));
    assert!(
        (a - b).abs() <= 1e-9 * a.abs().max(1.0),
        "{what}: grouped volume {b} disagrees with sequential {a}"
    );
}

/// Disjoint cutters: the grouped path fuses them into one boolean and must
/// still produce the sequential result.
#[test]
fn disjoint_cutters_agree_with_the_sequential_path() {
    let wall = boxx(2.0, 0.1, 0.0, 4.0, 0.2, 3.0, 0.0);
    let tools: Vec<_> = (0..6)
        .map(|i| boxx(0.5 + f64::from(i) * 0.6, 0.1, 0.5, 0.3, 0.5, 1.0, 0.0))
        .collect();

    let expected = sequential(&wall, &tools);
    let actual = BoolmeshBoolean::new()
        .subtract_many(&wall, &tools, &options())
        .expect("grouped");

    assert_same_volume(&expected, &actual, "disjoint");
    assert!(
        volume(&actual) < volume(&wall),
        "cutting must remove volume"
    );
}

/// Mutually overlapping cutters cannot be fused. The override must fall back
/// to per-tool subtraction rather than fusing them into an invalid union.
///
/// This is the correctness cliff: concatenating OVERLAPPING solids produces a
/// self-intersecting mesh, and subtracting that would give a wrong answer
/// while still looking like a valid result.
#[test]
fn overlapping_cutters_are_not_fused() {
    let wall = boxx(2.0, 0.1, 0.0, 4.0, 0.2, 3.0, 0.0);
    // Three concentric cutters: every pair overlaps.
    let tools: Vec<_> = [0.4, 0.8, 1.2]
        .into_iter()
        .map(|s| boxx(2.0, 0.1, 1.0, s, 0.5, s, 0.0))
        .collect();

    let expected = sequential(&wall, &tools);
    let actual = BoolmeshBoolean::new()
        .subtract_many(&wall, &tools, &options())
        .expect("grouped");

    assert_same_volume(&expected, &actual, "overlapping");
}

/// A mixed layout exercises both branches in one call: some cutters group,
/// others must stay separate.
#[test]
fn a_mixed_layout_groups_only_what_is_disjoint() {
    let wall = boxx(3.0, 0.1, 0.0, 6.0, 0.2, 3.0, 0.0);
    let tools = vec![
        boxx(1.0, 0.1, 0.5, 0.4, 0.5, 1.0, 0.0), // isolated
        boxx(3.0, 0.1, 0.5, 0.6, 0.5, 1.0, 0.0), // overlaps the next
        boxx(3.2, 0.1, 0.5, 0.6, 0.5, 1.0, 0.0), // overlaps the previous
        boxx(5.0, 0.1, 0.5, 0.4, 0.5, 1.0, 0.0), // isolated
    ];

    let expected = sequential(&wall, &tools);
    let actual = BoolmeshBoolean::new()
        .subtract_many(&wall, &tools, &options())
        .expect("grouped");

    assert_same_volume(&expected, &actual, "mixed");
}

/// Edge cases: no tools is the identity, one tool is a plain difference.
#[test]
fn empty_and_single_tool_batches_behave_like_the_default() {
    let wall = boxx(2.0, 0.1, 0.0, 4.0, 0.2, 3.0, 0.0);
    let provider = BoolmeshBoolean::new();

    let none = provider
        .subtract_many(&wall, &[], &options())
        .expect("no tools");
    assert_same_volume(&wall, &none, "empty batch");

    let tool = boxx(2.0, 0.1, 1.0, 0.5, 0.5, 1.0, 0.0);
    let one = provider
        .subtract_many(&wall, std::slice::from_ref(&tool), &options())
        .expect("one tool");
    let expected = sequential(&wall, std::slice::from_ref(&tool));
    assert_same_volume(&expected, &one, "single tool");
}

/// Randomised differential gate: grouped and sequential must agree on many
/// layouts, not just the three hand-picked ones.
///
/// Cutter positions are drawn so that overlaps occur by chance, which
/// exercises the grouping decision itself rather than a layout chosen to make
/// it look good.
#[test]
fn grouped_and_sequential_agree_across_random_layouts() {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1_u64 << 53) as f64
    };

    let provider = BoolmeshBoolean::new();
    let mut grouped_wins = 0usize;

    for case in 0..25 {
        let wall = boxx(3.0, 0.1, 0.0, 6.0, 0.2, 3.0, 0.0);
        let count = 2 + (case % 5);
        let tools: Vec<_> = (0..count)
            .map(|_| {
                // Positions spread over the wall; widths large enough that
                // neighbours sometimes collide and sometimes do not.
                let cx = 0.6 + next() * 4.8;
                let width = 0.2 + next() * 0.5;
                boxx(cx, 0.1, 0.5, width, 0.5, 1.0, 0.0)
            })
            .collect();

        let expected = sequential(&wall, &tools);
        let actual = provider
            .subtract_many(&wall, &tools, &options())
            .expect("grouped");
        assert_same_volume(&expected, &actual, &format!("random case {case}"));

        if actual.triangle_count() != wall.triangle_count() {
            grouped_wins += 1;
        }
    }

    assert!(
        grouped_wins >= 20,
        "most random cases must actually cut, else the gate proves nothing"
    );
}
