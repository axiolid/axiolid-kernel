#!/usr/bin/env python3
"""Mutation probes for the boolean contract (ADR 0017 sections 1-4).

Each probe injects one semantic defect and asserts the contract suites FAIL.
A surviving probe means the gate is blind to that defect and the tests are
decorative. Sources are restored byte-identically afterwards.
"""
import hashlib
import subprocess
import sys
from pathlib import Path

ROOT = Path("/mnt/backup/build-cache/axiolid-solibri-spatial")

SUITES = [
    ["cargo", "test", "-p", "axiolid-kernel", "--all-features", "--test", "boolean_contract"],
    ["cargo", "test", "-p", "axiolid-boolmesh", "--all-features", "--test", "symmetric_difference"],
    # `csg_deferral` was deleted when the deferral ended (ADR 0017 fully
    # landed); the conformance suite superseded it.
    ["cargo", "test", "-p", "axiolid-boolmesh", "--all-features", "--test", "conformance"],
]

# (name, relative path, find, replace)
PROBES = [
    (
        "ops-drop-symmetric-difference",
        "crates/foundation/core/src/operation.rs",
        "    pub const ALL: [Self; 4] = [\n        Self::Union,\n        Self::Intersection,\n        Self::Difference,\n        Self::SymmetricDifference,\n    ];",
        "    pub const ALL: [Self; 3] = [Self::Union, Self::Intersection, Self::Difference];",
    ),
    (
        "ops-difference-claims-commutative",
        "crates/foundation/core/src/operation.rs",
        "        !matches!(self, Self::Difference)",
        "        true",
    ),
    (
        "registry-skips-precondition-validation",
        "crates/contracts/common/src/boolean.rs",
        "        SolidRequirements::Oriented.validate_operands(subject, &[tool])?;",
        "        let _ = SolidRequirements::Oriented;",
    ),
    (
        "solid-accepts-inside-out",
        "crates/contracts/common/src/solid.rs",
        "        if six_volume < 0.0 {",
        "        if false {",
    ),
    (
        "solid-accepts-zero-volume",
        "crates/contracts/common/src/solid.rs",
        "        if six_volume == 0.0 {",
        "        if false {",
    ),
    (
        "cancellation-never-fires",
        "crates/contracts/common/src/cancel.rs",
        "        if self.is_cancelled() {\n            return Err(GeomError::Cancelled);\n        }",
        "        if false {\n            return Err(GeomError::Cancelled);\n        }",
    ),
    (
        "cancellation-token-clones-dont-share",
        "crates/contracts/common/src/cancel.rs",
        "        Arc::ptr_eq(&self.flag, &other.flag)",
        "        self.is_cancelled() == other.is_cancelled()",
    ),
    (
        "evidence-absorb-loses-sub-operations",
        "crates/contracts/common/src/evidence.rs",
        "        self.sub_operations += other.sub_operations;",
        "        self.sub_operations = other.sub_operations;",
    ),
    (
        "evidence-absorb-overwrites-input-counts",
        "crates/contracts/common/src/evidence.rs",
        "        self.output_triangles = other.output_triangles;",
        "        self.subject_triangles = other.subject_triangles;\n        self.output_triangles = other.output_triangles;",
    ),
    (
        "xor-reports-one-pass",
        "crates/contracts/common/src/boolean.rs",
        "    evidence.sub_operations = 3;",
        "    evidence.sub_operations = 1;",
    ),
    (
        "xor-skips-the-final-difference",
        "crates/contracts/common/src/boolean.rs",
        "    if intersection.mesh.indices.is_empty() {",
        "    if true {",
    ),
]


def run(cmd):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True).returncode


def suites_pass():
    return all(run(cmd) == 0 for cmd in SUITES)


def digest():
    files = sorted({ROOT / path for _, path, _, _ in PROBES})
    hasher = hashlib.sha256()
    for path in files:
        hasher.update(path.read_bytes())
    return hasher.hexdigest()


def main():
    before = digest()
    if not suites_pass():
        print("baseline: FAIL -- fix the suites before probing")
        return 1
    print("baseline: PASS\n")

    killed, leaked = [], []
    for name, rel, find, repl in PROBES:
        path = ROOT / rel
        original = path.read_text()
        if find not in original:
            print(f"  !! {name}: anchor not found in {rel}")
            leaked.append(name)
            continue
        if original.count(find) != 1:
            print(f"  !! {name}: anchor not unique in {rel}")
            leaked.append(name)
            continue
        try:
            path.write_text(original.replace(find, repl))
            if suites_pass():
                print(f"  LEAK    {name}")
                leaked.append(name)
            else:
                print(f"  killed  {name}")
                killed.append(name)
        finally:
            path.write_text(original)

    print(f"\nrestored byte-identical: {digest() == before}")
    print(f"killed {len(killed)}/{len(PROBES)}")
    if leaked:
        print("LEAKED PROBES (gate is blind to these defects):")
        for name in leaked:
            print(f"  - {name}")
        return 1
    print("all probes killed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
