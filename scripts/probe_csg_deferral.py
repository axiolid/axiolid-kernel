#!/usr/bin/env python3
"""Mutation probes for the ADR 0017 CSG deferral guard.

Each probe simulates one gap being closed (or the deferral being violated) and
asserts the guard suite FAILS. A surviving probe means the guard is blind.
"""
from __future__ import annotations

import hashlib
import subprocess
import sys
from pathlib import Path

ROOT = Path("/mnt/backup/build-cache/axiolid-solibri-spatial")
CRATES = ROOT / "crates"

# (name, relative path, anchor, replacement)
PROBES = [
    (
        "op-set-gains-symmetric-difference",
        "axiolid-core/src/operation.rs",
        "    Difference,\n}",
        "    Difference,\n    SymmetricDifference,\n}",
    ),
    (
        "scalar-gains-boolean-oracle",
        "axiolid-scalar/src/lib.rs",
        "#![forbid(unsafe_code)]",
        "#![forbid(unsafe_code)]\n// impl MeshBoolean for ScalarBoolean {}",
    ),
    (
        "kernel-gains-conformance-suite",
        "axiolid-kernel/src/boolean.rs",
        "//! Mesh boolean capability and executable provider registry.",
        "//! Mesh boolean capability and executable provider registry.\n"
        "// pub fn assert_mesh_boolean_conformance() {}",
    ),
    (
        "execution-gains-cancellation",
        "axiolid-kernel/src/execution.rs",
        "pub enum ScratchRequirement {",
        "pub struct CancellationToken;\npub enum ScratchRequirement {",
    ),
    (
        "kernel-gains-solid-validation",
        "axiolid-kernel/src/boolean.rs",
        "pub trait MeshBoolean: Backend {",
        "pub enum SolidValidation {}\npub trait MeshBoolean: Backend {",
    ),
    (
        "boolean-gains-evidence",
        "axiolid-kernel/src/boolean.rs",
        "#[derive(Debug, Clone)]\nstruct RegisteredBoolean {",
        "pub struct BooleanEvidence;\n#[derive(Debug, Clone)]\nstruct RegisteredBoolean {",
    ),
    (
        "native-cpp-backend-introduced",
        "axiolid-boolmesh/Cargo.toml",
        "[dependencies]",
        "[dependencies]\nmanifold3d = \"3.0\"",
    ),
]

GUARD = [
    "cargo", "test", "-p", "axiolid-core", "--test", "csg_deferral",
]
ENV_TOOLCHAIN = {"RUSTUP_TOOLCHAIN": "1.88.0"}


def digest() -> str:
    h = hashlib.sha256()
    for rel, _, _ in sorted({(p[1], 0, 0) for p in PROBES}):
        h.update((CRATES / rel).read_bytes())
    return h.hexdigest()


def run_guard() -> bool:
    """True when the guard suite passes."""
    import os

    env = dict(os.environ)
    env.update(ENV_TOOLCHAIN)
    proc = subprocess.run(
        GUARD, cwd=ROOT, env=env, capture_output=True, text=True
    )
    return proc.returncode == 0


def main() -> int:
    before = digest()

    if not run_guard():
        print("baseline: FAIL -- guard suite is red before mutation")
        return 1
    print("baseline: PASS\n")

    leaked: list[str] = []
    for name, rel, anchor, replacement in PROBES:
        path = CRATES / rel
        original = path.read_text()
        if anchor not in original:
            print(f"  !! {name}: anchor not found in {rel}")
            leaked.append(name)
            continue
        if original.count(anchor) != 1:
            print(f"  !! {name}: anchor is not unique in {rel}")
            leaked.append(name)
            continue

        path.write_text(original.replace(anchor, replacement, 1))
        try:
            still_green = run_guard()
        finally:
            path.write_text(original)

        if still_green:
            print(f"  LEAKED  {name}")
            leaked.append(name)
        else:
            print(f"  killed  {name}")

    restored = digest() == before
    print(f"\nrestored byte-identical: {restored}")
    print(f"killed {len(PROBES) - len(leaked)}/{len(PROBES)}")

    if leaked:
        print("LEAKED PROBES (guard is blind to these):")
        for name in leaked:
            print(f"  - {name}")
        return 1
    if not restored:
        print("SOURCES NOT RESTORED")
        return 1
    print("all probes killed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
