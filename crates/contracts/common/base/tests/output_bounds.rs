//! Contract gates for output bounds and the batch-into seam.

use axiolid_contracts::OutputBound;

/// A bounded operation lets a caller size the destination before running.
#[test]
fn bounded_operations_are_preallocatable() {
    assert_eq!(OutputBound::OneToOne.upper_bound(10), Some(10));
    assert_eq!(OutputBound::AtMost { max: 3 }.upper_bound(10), Some(30));
    assert_eq!(OutputBound::Unbounded.upper_bound(10), None);

    assert!(OutputBound::OneToOne.is_preallocatable(10));
    assert!(!OutputBound::Unbounded.is_preallocatable(10));
}

/// Overflow must not wrap into a small allocation. A wrapped bound would
/// under-allocate the destination and corrupt every write past the wrap.
#[test]
fn an_overflowing_bound_reports_unbounded_not_a_wrapped_value() {
    assert_eq!(
        OutputBound::AtMost { max: usize::MAX }.upper_bound(2),
        None,
        "must not wrap"
    );
    assert!(!OutputBound::AtMost { max: usize::MAX }.is_preallocatable(2));
}

/// The scan: per-element counts become disjoint write offsets plus a total.
/// This is what removes the atomic counter from the hot path.
#[test]
fn the_scan_turns_counts_into_disjoint_write_offsets() {
    // The 0/2/1/0/3 example: five triangles, six outputs.
    let counts = [0, 2, 1, 0, 3];
    let (offsets, total) = OutputBound::AtMost { max: 3 }
        .write_offsets(&counts)
        .expect("bounded counts scan");

    assert_eq!(offsets, vec![0, 0, 2, 3, 3]);
    assert_eq!(total, 6);

    // Every element's slice is disjoint and inside the total.
    for (index, &count) in counts.iter().enumerate() {
        let end = offsets[index] + count;
        assert!(end <= total, "element {index} writes past the destination");
        for (other, &other_count) in counts.iter().enumerate() {
            if other == index || count == 0 || other_count == 0 {
                continue;
            }
            let other_end = offsets[other] + other_count;
            let overlaps = offsets[index] < other_end && offsets[other] < end;
            assert!(!overlaps, "elements {index} and {other} overlap");
        }
    }
}

/// A provider exceeding its declared bound is a contract violation, caught by
/// the scan rather than by a buffer overrun at write time.
#[test]
fn counts_exceeding_the_declared_bound_are_rejected() {
    assert_eq!(OutputBound::AtMost { max: 2 }.write_offsets(&[1, 3]), None);
    assert_eq!(OutputBound::OneToOne.write_offsets(&[1, 2]), None);
    assert!(OutputBound::OneToOne.write_offsets(&[1, 1, 0]).is_some());
}

/// An unbounded operation still scans: the offsets are valid, there is just no
/// per-element ceiling to check against.
#[test]
fn an_unbounded_operation_still_produces_valid_offsets() {
    let (offsets, total) = OutputBound::Unbounded
        .write_offsets(&[5, 0, 7])
        .expect("unbounded still scans");
    assert_eq!(offsets, vec![0, 5, 5]);
    assert_eq!(total, 12);
}

/// An overflowing total must fail the scan, not wrap.
#[test]
fn an_overflowing_total_fails_the_scan() {
    assert_eq!(
        OutputBound::Unbounded.write_offsets(&[usize::MAX, 1]),
        None,
        "must not wrap the running total"
    );
}

/// Empty input is a valid, empty plan.
#[test]
fn an_empty_batch_scans_to_an_empty_plan() {
    let (offsets, total) = OutputBound::OneToOne.write_offsets(&[]).expect("empty");
    assert!(offsets.is_empty());
    assert_eq!(total, 0);
}
