# 0041 — CMake and native release bundles

- **Status:** Accepted
- **Date:** 2026-09-03
- **Deciders:** Friedrich Schrödter
- **Supersedes:** —

## Context

C and C++ consumers need one target name regardless of whether Axiolid is built from an immutable source revision or consumed as a release archive. Cargo outputs alone do not carry CMake usage requirements, runtime-library placement, target provenance, or integrity metadata.

## Decision

Expose `Axiolid::axiolid` from both paths. Source consumption goes through `axiolid_fetch(...)`, which accepts only a full 40-character Git commit. Binary archives contain both shared and static libraries, the generated header, package config files, the MPL-2.0 license, and a hash-bearing manifest. Archives and their sidecars are reproducible and SHA-256 verified before release upload.

`AXIOLID_LINKAGE=SHARED|STATIC` selects the implementation behind the stable target. Shared is the default because it minimizes platform-specific transitive system-link requirements. ABI compatibility is the C ABI v0.4 contract, not the Rust crate ABI.

## Consequences

- Mutable branches and tags are rejected by the source helper.
- Release support is a closed target matrix; unknown target triples fail packaging.
- Cross-built AArch64 Linux archives are format- and architecture-verified but not executed on the x86 runner.
- Adding a target requires packager layout support, binary-format verification, CMake metadata, CI evidence, and documentation.

## Rejected alternatives

- **Expose raw Cargo artifact paths:** no package discovery, provenance, or stable usage requirements.
- **Download “latest” source:** not reproducible and vulnerable to ref movement.
- **Ship only shared or only static:** prevents downstreams with deployment constraints from choosing explicitly.
