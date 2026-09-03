# 0042 — Downstream compatibility is an executable black-box gate

- **Status:** Accepted
- **Date:** 2026-09-03
- **Deciders:** Friedrich Schrödter
- **Supersedes:** —

## Context

Workspace builds can accidentally supply path dependencies, unified features, unpublished packages, generated headers, or sibling build artifacts. Those tests do not prove that a clean application can use the public Rust and native boundaries. The native release archive also needs an executable consumer, not only structural checksum validation.

Crates.io publication ordering and public version release are owned by the release workflow. The compatibility gate must still exercise the exact candidate commit before publication without pretending that an unpublished crate already exists in the registry.

## Decision

Downstream compatibility is gated by disposable consumers copied outside the workspace before dependency resolution or compilation.

- Rust closure and facade probes replace their development-only path dependencies with exact-version Git dependencies pinned to one 40-hex commit in a temporary bare source artifact. Resolution is inspected to reject workspace sources or mutable Axiolid dependencies.
- Native C and C++ probes consume the extracted, checksum-verified archive through `find_package(Axiolid 0.4 CONFIG REQUIRED)` and `Axiolid::axiolid`. They execute semantic success paths and assert the typed `UnsupportedExact` refusal.
- Probe policy rejects unpublished direct packages and unversioned native ABI names.
- Mutations remove the facade feature, an allowed Rust package, a native symbol, the public header, and the CMake package configuration. Each mutation must make its corresponding gate fail.
- Linux, macOS, and Windows run the Rust probes and the CMake Debug/Release, shared/static archive probes in CI.

The exact Git source artifact is a pre-publication compatibility mechanism, not a claim that crates have been released. Registry packaging and publication remain release-workflow responsibilities.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep only workspace integration tests | Path visibility and feature unification can hide downstream failures. |
| Depend on mutable `main` or a branch | The evidence cannot be tied to the reviewed candidate. |
| Publish test versions to crates.io | It creates irreversible public state during ordinary compatibility CI and duplicates release ownership. |
| Compile native examples against the sibling build tree | It bypasses the archive manifest, exported CMake targets, installed header, and runtime layout. |

## Consequences

**Positive**

- The supported Rust and native entry points are exercised from clean consumer roots.
- Evidence is tied to an immutable commit and verified native archive.
- Required-package, feature, header, and symbol gates are mutation-proven.

**Negative / costs**

- Git-sourced Rust dependencies compile as distinct Cargo source identities and increase CI time.
- Pre-publication probes do not prove crates.io availability; release automation must add that evidence.

**Follow-ups / risks to watch**

- The v0.4 release gate must rerun the same consumers against published crate versions and attached native archives.
- Keep direct package allowlists derived from publishable `crates/` members so internal tools cannot leak into examples.

## Relation to existing code

- `scripts/test-downstream-consumers.py`
- `scripts/test-native-cmake.py`
- `tests/consumers/`
- `tests/native/cmake-consumer/`
- `.github/workflows/native.yml`
