#!/usr/bin/env python3
"""Mutation probes for tessellation and interference."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SUITES = [
    ["cargo", "test", "-p", "axiolid-scalar", "--all-features", "--test", "tessellate"],
    ["cargo", "test", "-p", "axiolid-scalar", "--all-features", "--test", "clash"],
]

PROBES = [
    (
        "tess-ignores-the-chord-budget",
        "crates/axiolid-scalar/src/tessellate.rs",
        "Some(s) if s > budget.chord_tolerance => {}",
        "Some(s) if s > budget.chord_tolerance * 1.0e12 => {}",
    ),
    (
        "tess-cell-diagonal-choice-inverted",
        "crates/axiolid-scalar/src/tessellate.rs",
        "if shorter_diagonal_is_ad(&positions, a, b, c, d) {",
        "if !shorter_diagonal_is_ad(&positions, a, b, c, d) {",
    ),
    (
        "tess-sagitta-measured-to-the-start-not-the-chord",
        "crates/axiolid-scalar/src/tessellate.rs",
        "let deviation = point_to_segment(pm, pa, pb);",
        "let deviation = (pm - pa).length();",
    ),
    (
        "tess-refinement-never-doubles",
        "crates/axiolid-scalar/src/tessellate.rs",
        "n = (2 * (n - 1) + 1).min(budget.max_samples_per_direction);",
        "n = (n + 1).min(budget.max_samples_per_direction);",
    ),
    (
        "tess-exhaustion-is-hidden",
        "crates/axiolid-scalar/src/tessellate.rs",
        "nu >= budget.max_samples_per_direction || nv >= budget.max_samples_per_direction",
        "false",
    ),
    (
        "tess-accepts-a-single-sample",
        "crates/axiolid-scalar/src/tessellate.rs",
        "if max_samples_per_direction < 2 {",
        "if max_samples_per_direction < 1 {",
    ),
    # `tess-projection-unclamped` was removed after measurement rather than
    # left as a permanent leak. The sagitta probe evaluates the MIDPOINT of a
    # chord; a midpoint's projection foot always lies within its own segment,
    # so the clamp cannot be reached from `tessellate_patch`. It is defensive
    # code for future callers that pass an arbitrary point, not a live branch,
    # and a probe that no reachable input can kill is noise in the report.
]

CLASH_PROBES = [
    (
        "clash-broad-phase-never-rejects",
        "crates/axiolid-scalar/src/clash.rs",
        "if !box_a.intersects(box_b) {",
        "if false {",
    ),
    (
        "clash-proper-crossing-is-not-penetration",
        "crates/axiolid-scalar/src/clash.rs",
        "report.penetrating_pairs.push((i, j));\n                    report.kind = Interference::Penetrating;",
        "report.penetrating_pairs.push((i, j));\n                    report.kind = Interference::Touching;",
    ),
    (
        "clash-containment-never-checked",
        "crates/axiolid-scalar/src/clash.rs",
        "if report.kind != Interference::Penetrating && !a.indices.is_empty() && !b.indices.is_empty() {",
        "if false && !a.indices.is_empty() && !b.indices.is_empty() {",
    ),
    (
        "clash-boundary-counts-as-inside",
        "crates/axiolid-scalar/src/clash.rs",
        "Some(w > 0.75)",
        "Some(w > 0.25)",
    ),
    (
        "clash-coplanar-always-overlaps",
        "crates/axiolid-scalar/src/clash.rs",
        "if coplanar_pair_overlaps(ta, tb) {",
        "if true {",
    ),
    (
        "clash-coplanar-never-overlaps",
        "crates/axiolid-scalar/src/clash.rs",
        "if coplanar_pair_overlaps(ta, tb) {",
        "if false {",
    ),
    (
        "clash-touching-reported-as-clear",
        "crates/axiolid-scalar/src/clash.rs",
        "if report.kind == Interference::Clear {\n                        report.kind = Interference::Touching;\n                    }\n                }\n                // `Coplanar` short-circuits",
        "if false {\n                        report.kind = Interference::Touching;\n                    }\n                }\n                // `Coplanar` short-circuits",
    ),
]

PROBES = PROBES + CLASH_PROBES


def run(cmd):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True).returncode


def suites_pass():
    return all(run(c) == 0 for c in SUITES)


def main():
    originals = {}
    for _, rel, _, _ in PROBES:
        p = ROOT / rel
        originals[rel] = p.read_text()

    if not suites_pass():
        print("baseline: FAIL -- fix the suites before probing")
        return 1
    print("baseline: PASS\n")

    killed, leaked = 0, []
    try:
        for name, rel, find, replace in PROBES:
            p = ROOT / rel
            src = originals[rel]
            if find not in src:
                print(f"  ERROR   {name}: anchor not found in {rel}")
                leaked.append(name)
                continue
            p.write_text(src.replace(find, replace, 1))
            try:
                if suites_pass():
                    print(f"  LEAK    {name}")
                    leaked.append(name)
                else:
                    print(f"  killed  {name}")
                    killed += 1
            finally:
                p.write_text(src)
    finally:
        for rel, text in originals.items():
            (ROOT / rel).write_text(text)

    restored = all((ROOT / rel).read_text() == text for rel, text in originals.items())
    print(f"\nrestored byte-identical: {restored}")
    print(f"killed {killed}/{len(PROBES)}")
    if leaked:
        print("LEAKED PROBES (gate is blind to these defects):")
        for name in leaked:
            print(f"  - {name}")
        return 1
    print("all probes killed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
