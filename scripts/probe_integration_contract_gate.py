#!/usr/bin/env python3
"""Prove the downstream-contract freshness gate detects capability drift."""

from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "crates/contracts/common/base/src/capability_id.rs"
CHECK = ROOT / "scripts/check-integration-contract.sh"
NEEDLE = b"        MESH_BOOLEAN,\n"


def run_check() -> int:
    return subprocess.run(
        [str(CHECK)],
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    ).returncode


before = SOURCE.read_bytes()
if run_check() != 0:
    raise SystemExit("baseline integration contract gate is not green")
if before.count(NEEDLE) != 1:
    raise SystemExit("mutation target is absent or ambiguous")

try:
    SOURCE.write_bytes(before.replace(NEEDLE, b"", 1))
    if SOURCE.read_bytes() == before:
        raise SystemExit("mutation did not land")
    if run_check() == 0:
        raise SystemExit("gate missed a removed promised capability identifier")
finally:
    SOURCE.write_bytes(before)

if SOURCE.read_bytes() != before:
    raise SystemExit("mutation restoration failed")
if run_check() != 0:
    raise SystemExit("restored integration contract gate is not green")
print("integration contract mutation detected and source restored")
