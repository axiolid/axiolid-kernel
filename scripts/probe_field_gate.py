#!/usr/bin/env python3
"""Mutation probes for the axiolid-field gates.

Each probe injects one semantic defect into the crate source and asserts that
the field test suites FAIL. A probe that survives means the suite cannot
detect that defect, so the gate is decorative. Sources are restored verbatim
from an in-memory snapshot, and the restoration is verified by hash.
"""

from __future__ import annotations

import hashlib
import subprocess
import sys
from pathlib import Path

ROOT = Path("/mnt/backup/build-cache/axiolid-solibri-spatial")
SRC = ROOT / "crates/algorithms/sampled/field/src"

# (name, file, old, new) -- each must change real behaviour, not a no-op.
PROBES = [
    (
        "coverage-fabricates-occupancy",
        "sample.rs",
        "LayeredCell::with_layers(hits, Vec::new())?",
        "LayeredCell::with_layers(hits.clone(), hits.first().map(|h| vec![axiolid_core::Interval::new(h.w(), h.w() + 1.0)]).unwrap_or_default())?",
    ),
    (
        "coverage-drops-stacked-layers",
        "sample.rs",
        "evidence.surface_hits += hits.len();",
        "hits.truncate(1);\n            evidence.surface_hits += hits.len();",
    ),
    (
        "cell-merges-touching-occupancy",
        "cell.rs",
        ".any(|pair| pair[0].end >= pair[1].start)",
        ".any(|pair| pair[0].end > pair[1].start)",
    ),
    (
        "config-ignores-cell-budget",
        "config.rs",
        "if cells > budget.max_cells {",
        "if false && cells > budget.max_cells {",
    ),
    (
        "config-accepts-left-handed-frame",
        "config.rs",
        "&& frame.x.cross(frame.y).dot(frame.z) > 0.0",
        "&& frame.x.cross(frame.y).dot(frame.z) != 0.0",
    ),
    (
        "morphology-radius-becomes-cell-count",
        "morphology.rs",
        "let reach = radius / config.cell_size();",
        "let reach = radius;",
    ),
    (
        "clearance-includes-reference-surface",
        "clearance.rs",
        "*value > w + linear",
        "*value > w - 1.0e9",
    ),
    (
        "navigation-ignores-max-step",
        "navigate.rs",
        "if rise > envelope.max_step + linear {",
        "if false && rise > envelope.max_step + linear {",
    ),
    (
        "navigation-ignores-agent-height",
        "navigate.rs",
        "if report.distance + linear < envelope.agent_height {",
        "if false && report.distance + linear < envelope.agent_height {",
    ),
    (
        "occupancy-accepts-unbalanced-crossings",
        "cell.rs",
        "if self.surfaces.len() % 2 != 0 {\n            return Err(LayeredFieldError::UnbalancedCrossings);\n        }",
        "if false {\n            return Err(LayeredFieldError::UnbalancedCrossings);\n        }",
    ),
]

TEST_CMD = [
    "cargo", "+1.88.0", "test", "-p", "axiolid-field",
    "--all-features", "--quiet",
]


def digest(paths: dict[str, str]) -> str:
    h = hashlib.sha256()
    for name in sorted(paths):
        h.update(name.encode())
        h.update(paths[name].encode())
    return h.hexdigest()


def main() -> int:
    files = {p.name: p.read_text() for p in SRC.glob("*.rs")}
    baseline = digest(files)

    # A mutation probe is only meaningful if the unmutated suite passes.
    clean = subprocess.run(TEST_CMD, cwd=ROOT, capture_output=True, text=True)
    if clean.returncode != 0:
        print("BASELINE FAILED - fix the suite before probing")
        print(clean.stdout[-3000:])
        print(clean.stderr[-3000:])
        return 2
    print("baseline: PASS\n")

    killed, leaked = [], []
    try:
        for name, filename, old, new in PROBES:
            target = SRC / filename
            original = files[filename]
            if old not in original:
                print(f"  !! {name}: anchor not found in {filename}")
                leaked.append(name)
                continue
            if original.count(old) != 1:
                print(f"  !! {name}: anchor is not unique in {filename}")
                leaked.append(name)
                continue

            target.write_text(original.replace(old, new, 1))
            result = subprocess.run(TEST_CMD, cwd=ROOT, capture_output=True, text=True)
            target.write_text(original)

            if result.returncode != 0:
                print(f"  killed  {name}")
                killed.append(name)
            else:
                print(f"  LEAKED  {name}")
                leaked.append(name)
    finally:
        for filename, text in files.items():
            (SRC / filename).write_text(text)

    restored = digest({p.name: p.read_text() for p in SRC.glob("*.rs")})
    print(f"\nrestored byte-identical: {restored == baseline}")
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
