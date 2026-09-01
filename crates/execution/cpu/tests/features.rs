//! Feature-selection logic for the CPU context.
//!
//! Context construction and pool sizing are covered by inline tests in
//! `src/execution.rs`. What is pinned here is `CpuFeatures` selection, which
//! those tests do not reach.
//!
//! These tests construct `CpuFeatures` explicitly rather than calling
//! `detect()`, so they assert the same thing on every host. A test whose
//! outcome depends on the build machine's CPU would pass or fail for reasons
//! unrelated to the code.

use axiolid_backend_cpu::{CpuFeatures, CpuInstructionSet};

const NONE: CpuFeatures = CpuFeatures {
    sse42: false,
    avx2_fma: false,
    avx512: false,
    neon: false,
};

/// The portable path is always available, whatever the hardware reports.
///
/// It is the fallback the whole design rests on: a binary must stay runnable
/// on a machine that supports no specialization at all.
#[test]
fn the_portable_path_is_always_supported() {
    assert!(NONE.supports(CpuInstructionSet::Portable));

    let everything = CpuFeatures {
        sse42: true,
        avx2_fma: true,
        avx512: true,
        neon: true,
    };
    assert!(
        everything.supports(CpuInstructionSet::Portable),
        "portable stays available even when every specialization is present"
    );
}

/// A feature that is absent is never reported as supported.
///
/// Claiming an unavailable instruction set would produce an illegal
/// instruction at runtime rather than a graceful fallback.
#[test]
fn absent_features_are_never_supported() {
    for set in [
        CpuInstructionSet::Sse42,
        CpuInstructionSet::Avx2,
        CpuInstructionSet::Avx512,
        CpuInstructionSet::Neon,
    ] {
        assert!(
            !NONE.supports(set),
            "{set:?} must not be supported when no features are detected"
        );
    }
}

/// Each flag gates exactly its own instruction set.
#[test]
fn each_flag_gates_only_its_own_instruction_set() {
    let cases = [
        (
            CpuFeatures {
                sse42: true,
                ..NONE
            },
            CpuInstructionSet::Sse42,
        ),
        (
            CpuFeatures {
                avx2_fma: true,
                ..NONE
            },
            CpuInstructionSet::Avx2,
        ),
        (
            CpuFeatures {
                avx512: true,
                ..NONE
            },
            CpuInstructionSet::Avx512,
        ),
        (CpuFeatures { neon: true, ..NONE }, CpuInstructionSet::Neon),
    ];
    for (features, enabled) in cases {
        for set in [
            CpuInstructionSet::Sse42,
            CpuInstructionSet::Avx2,
            CpuInstructionSet::Avx512,
            CpuInstructionSet::Neon,
        ] {
            assert_eq!(
                features.supports(set),
                set == enabled,
                "{features:?} should support {enabled:?} and nothing else, but disagreed on {set:?}"
            );
        }
    }
}

/// `best` prefers the widest available x86 path, in descending order.
#[test]
fn best_prefers_the_widest_available_x86_path() {
    assert_eq!(NONE.best(), CpuInstructionSet::Portable);

    assert_eq!(
        CpuFeatures {
            sse42: true,
            ..NONE
        }
        .best(),
        CpuInstructionSet::Sse42
    );
    assert_eq!(
        CpuFeatures {
            sse42: true,
            avx2_fma: true,
            ..NONE
        }
        .best(),
        CpuInstructionSet::Avx2
    );
    assert_eq!(
        CpuFeatures {
            sse42: true,
            avx2_fma: true,
            avx512: true,
            ..NONE
        }
        .best(),
        CpuInstructionSet::Avx512,
        "AVX-512 outranks AVX2 and SSE4.2 when all are present"
    );
}

/// `best` never names an instruction set it would not also support.
///
/// The two functions are separate, so they could disagree. A `best` that
/// returned an unsupported set would be selected and then executed illegally.
#[test]
fn best_always_names_a_supported_instruction_set() {
    for bits in 0..16_u8 {
        let features = CpuFeatures {
            sse42: bits & 1 != 0,
            avx2_fma: bits & 2 != 0,
            avx512: bits & 4 != 0,
            neon: bits & 8 != 0,
        };
        let best = features.best();
        assert!(
            features.supports(best),
            "{features:?} chose {best:?}, which it does not support"
        );
    }
}

/// NEON is chosen only when no x86 path is available.
///
/// The two families are mutually exclusive on real hardware, but the struct
/// can represent both. Pinning the order documents which wins rather than
/// leaving it to reading order in the `if` chain.
#[test]
fn neon_is_the_last_resort_before_portable() {
    assert_eq!(
        CpuFeatures { neon: true, ..NONE }.best(),
        CpuInstructionSet::Neon
    );
    assert_eq!(
        CpuFeatures {
            sse42: true,
            neon: true,
            ..NONE
        }
        .best(),
        CpuInstructionSet::Sse42,
        "an available x86 path is preferred over NEON when both are claimed"
    );
}
