//! Validation contracts for tessellation quality policy.
//!
//! `TessellationOptions` is the crate's only behaviour: it refuses limits that
//! would make approximation meaningless. There is deliberately no default
//! chord error, because acceptable error depends on source units and on what
//! the result is used for.

use axiolid_core::Tolerance;
use axiolid_tessellation_contract::{InvalidTessellationOptions, TessellationOptions};

fn tolerance() -> Tolerance {
    Tolerance::new(1.0e-6, 1.0e-9).expect("valid tolerance")
}

/// Valid limits are stored verbatim and read back unchanged.
#[test]
fn valid_limits_round_trip() {
    let options = TessellationOptions::new(0.01, 0.5, tolerance()).expect("valid options");
    assert_eq!(options.chord_error(), 0.01);
    assert_eq!(options.maximum_angle(), 0.5);
    assert_eq!(
        options.maximum_edge_length(),
        None,
        "edge length stays absent until explicitly added"
    );
}

/// A non-positive or non-finite chord error is refused.
///
/// Zero would demand infinite refinement and NaN would silently disable the
/// comparison, so both are rejected at construction rather than producing an
/// unbounded or vacuous tessellation later.
#[test]
fn degenerate_chord_error_is_refused() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            TessellationOptions::new(bad, 0.5, tolerance()),
            Err(InvalidTessellationOptions),
            "chord error {bad} must be refused"
        );
    }
}

/// The same for the maximum angle.
#[test]
fn degenerate_maximum_angle_is_refused() {
    for bad in [0.0, -0.5, f64::NAN, f64::INFINITY] {
        assert_eq!(
            TessellationOptions::new(0.01, bad, tolerance()),
            Err(InvalidTessellationOptions),
            "maximum angle {bad} must be refused"
        );
    }
}

/// The optional edge-length cap validates on the same terms.
///
/// It is optional, but "absent" and "present but nonsensical" are different:
/// supplying a bad value is an error rather than a silent fallback to absent.
#[test]
fn maximum_edge_length_is_optional_but_validated_when_present() {
    let base = TessellationOptions::new(0.01, 0.5, tolerance()).expect("valid options");

    let capped = base
        .with_maximum_edge_length(2.5)
        .expect("positive finite cap is accepted");
    assert_eq!(capped.maximum_edge_length(), Some(2.5));

    for bad in [0.0, -2.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            base.with_maximum_edge_length(bad),
            Err(InvalidTessellationOptions),
            "edge length {bad} must be refused, not silently dropped"
        );
    }
}

/// Adding a cap leaves the other limits untouched.
#[test]
fn adding_an_edge_length_cap_preserves_the_other_limits() {
    let base = TessellationOptions::new(0.02, 0.25, tolerance()).expect("valid options");
    let capped = base.with_maximum_edge_length(1.0).expect("valid cap");
    assert_eq!(capped.chord_error(), base.chord_error());
    assert_eq!(capped.maximum_angle(), base.maximum_angle());
    assert_eq!(capped.tolerance(), base.tolerance());
}

/// The error type carries a message naming the constraint that failed.
///
/// A bare unit error would tell a caller only that something was wrong; the
/// Display text is part of what makes the refusal actionable.
#[test]
fn the_refusal_explains_itself() {
    let message = InvalidTessellationOptions.to_string();
    assert!(
        message.contains("finite") && message.contains("positive"),
        "refusal must name the constraint, got {message:?}"
    );
}
