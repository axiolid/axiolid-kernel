#!/usr/bin/env python3
"""Mutation probes for curve evaluation and extrusion volume (C1 + C3).

The curve evaluator is now the thing profile flattening trusts, and the
extrusion identity `volume == area * depth` is the claim that makes extrusion
correct rather than merely closed. Both need proof their gates can fail.

Each probe injects one semantic defect and asserts the suites FAIL.
A surviving probe means the gate is blind to that defect.
"""

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SUITES = [
    ["cargo", "test", "-p", "axiolid-scalar", "--all-features", "--test", "curve"],
    ["cargo", "test", "-p", "axiolid-compile", "--all-features", "--test", "extrusion_volume"],
    ["cargo", "test", "-p", "axiolid-compile", "--all-features", "--test", "extrusion"],
]

# (name, relative path, find, replace)
PROBES = [
    # --- C1: evaluation -----------------------------------------------------
    (
        "circle-swaps-sin-for-cos",
        "crates/algorithms/reference/src/curve.rs",
        "frame.origin + frame.x * (rx * t.cos()) + frame.y * (ry * t.sin())\n}\n\nfn conic_tangent2",
        "frame.origin + frame.x * (rx * t.sin()) + frame.y * (ry * t.cos())\n}\n\nfn conic_tangent2",
    ),
    (
        "circle-tangent-loses-its-sign",
        "crates/algorithms/reference/src/curve.rs",
        "frame.x * (-rx * t.sin()) + frame.y * (ry * t.cos())\n}\n\nfn conic_point3",
        "frame.x * (rx * t.sin()) + frame.y * (ry * t.cos())\n}\n\nfn conic_point3",
    ),
    (
        "ellipse-collapses-to-a-circle",
        "crates/algorithms/reference/src/curve.rs",
        "Curve2::Ellipse(e) => Ok(conic_point2(&e.frame, e.semi_axis_x, e.semi_axis_y, t)),",
        "Curve2::Ellipse(e) => Ok(conic_point2(&e.frame, e.semi_axis_x, e.semi_axis_x, t)),",
    ),
    (
        "line-ignores-its-parameter",
        "crates/algorithms/reference/src/curve.rs",
        "    origin + direction * t\n}",
        "    origin + direction * 0.0\n}",
    ),
    (
        "polyline-drops-the-local-fraction",
        "crates/algorithms/reference/src/curve.rs",
        "    Ok(points[i] + (points[j] - points[i]) * local)",
        "    Ok(points[i] + (points[j] - points[i]) * 0.0)",
    ),
    (
        "polyline-wrap-ignores-closure",
        "crates/algorithms/reference/src/curve.rs",
        "    let next = (index + 1) % count;",
        "    let next = (index + 1).min(count - 1);",
    ),
    # --- C1: de Boor --------------------------------------------------------
    (
        "de-boor-projects-before-interpolating",
        "crates/algorithms/reference/src/curve.rs",
        "        work.push(core::array::from_fn(|k| c[k] * w));",
        "        work.push(core::array::from_fn(|k| c[k]));",
    ),
    (
        "de-boor-alpha-inverted",
        "crates/algorithms/reference/src/curve.rs",
        "                work[j][k] = work[j - 1][k] * (1.0 - alpha) + work[j][k] * alpha;\n            }\n            weights[j] = weights[j - 1] * (1.0 - alpha) + weights[j] * alpha;\n        }\n    }\n\n    let w = weights[d];",
        "                work[j][k] = work[j - 1][k] * alpha + work[j][k] * (1.0 - alpha);\n            }\n            weights[j] = weights[j - 1] * (1.0 - alpha) + weights[j] * alpha;\n        }\n    }\n\n    let w = weights[d];",
    ),
    (
        "de-boor-skips-the-rational-divide",
        "crates/algorithms/reference/src/curve.rs",
        "    Ok(from(core::array::from_fn(|k| work[d][k] / w)))",
        "    Ok(from(core::array::from_fn(|k| work[d][k])))",
    ),
    (
        "hodograph-drops-the-degree-factor",
        "crates/algorithms/reference/src/curve.rs",
        "            d as Scalar / denom",
        "            1.0 / denom",
    ),
    (
        "rational-derivative-skips-the-quotient-rule",
        "crates/algorithms/reference/src/curve.rs",
        "        (da[k] - (a[k] / w) * dw) / w",
        "        da[k] / w",
    ),
    (
        "degenerate-spline-is-extrapolated",
        "crates/algorithms/reference/src/curve.rs",
        "    if !matches!(hi.partial_cmp(&lo), Some(core::cmp::Ordering::Greater)) {",
        "    if false {",
    ),
    # --- C1: flattening -----------------------------------------------------
    (
        "flatten-ignores-the-chord-budget",
        "crates/algorithms/reference/src/curve.rs",
        "    if depth == 0 || sagitta2(pa, pb, pm) <= tol {",
        "    if depth == 0 || sagitta2(pa, pb, pm) <= tol * 1e9 {",
    ),
    (
        "sagitta-ignores-the-chord-and-measures-to-its-start",
        "crates/algorithms/reference/src/curve.rs",
        "    let t = ((m - a).dot(ab) / len2).clamp(0.0, 1.0);\n    (m - (a + ab * t)).length()",
        "    let _ = (len2, ab);\n    (m - a).length()",
    ),
    (
        "flatten-accepts-a-non-positive-tolerance",
        "crates/algorithms/reference/src/curve.rs",
        "chord_tolerance.is_sign_positive()",
        "chord_tolerance.is_sign_negative()",
    ),
    (
        "polyline-domain-guard-removed",
        "crates/algorithms/reference/src/curve.rs",
        "        if natural.end > 1.0 && requested <= 1.0 {",
        "        if false {",
    ),
    # --- C3: extrusion ------------------------------------------------------
    (
        "extrusion-drops-the-bottom-cap",
        "crates/execution/compile/src/extrude.rs",
        "        indices.extend_from_slice(&[t[0], t[2], t[1]]);",
        "        let _ = t;",
    ),
    (
        "extrusion-bottom-cap-not-reversed",
        "crates/execution/compile/src/extrude.rs",
        "        indices.extend_from_slice(&[t[0], t[2], t[1]]);",
        "        indices.extend_from_slice(&[t[0], t[1], t[2]]);",
    ),
    (
        "extrusion-side-quad-half-missing",
        "crates/execution/compile/src/extrude.rs",
        "            indices.extend_from_slice(&[a, b + top, a + top]);",
        "            let _ = (a, b);",
    ),
    (
        "extrusion-ignores-depth",
        "crates/execution/compile/src/extrude.rs",
        "    let offset = direction.normalize() * depth;",
        "    let offset = direction.normalize() * 1.0;",
    ),
    (
        "circle-ring-keeps-its-duplicate-vertex",
        "crates/execution/compile/src/profile.rs",
        "    ring.pop();\n    Ok(ring)\n}",
        "    Ok(ring)\n}",
    ),
]


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
