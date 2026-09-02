#!/usr/bin/env python3
"""Verify all publishable workspace crates before any crates.io upload.

Cargo's normal workspace packaging resolves path dependencies through crates.io.
That is impossible for a first multi-crate release because the internal package
names do not exist there yet. This verifier supplies command-scoped local
patches while Cargo creates and compiles the real normalized `.crate` archives.
The patches never enter a published manifest; version requirements remain the
registry contract.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def cargo_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    return json.loads(result.stdout)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--allow-dirty",
        action="store_true",
        help="forward --allow-dirty for local pre-commit verification",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="copy the exact verified .crate archives into this directory",
    )
    args = parser.parse_args()

    metadata = cargo_metadata()
    workspace_ids = set(metadata["workspace_members"])
    members = [package for package in metadata["packages"] if package["id"] in workspace_ids]
    members.sort(key=lambda package: package["name"])

    publishable = [package for package in members if package.get("publish") != []]
    excluded = [package for package in members if package.get("publish") == []]

    with tempfile.TemporaryDirectory(prefix="axiolid-package-verify-") as temporary:
        temporary_path = Path(temporary)
        patch_config = temporary_path / "workspace-patches.toml"
        target = temporary_path / "target"

        rows = ["[patch.crates-io]"]
        for package in publishable:
            package_path = Path(package["manifest_path"]).parent
            rows.append(f'"{package["name"]}" = {{ path = "{package_path}" }}')
        patch_config.write_text("\n".join(rows) + "\n", encoding="utf-8")

        command = ["cargo", "package", "--workspace", "--locked", "--quiet"]
        for package in excluded:
            command.extend(["--exclude", package["name"]])
        if args.allow_dirty:
            command.append("--allow-dirty")
        command.extend(["--config", str(patch_config)])

        environment = os.environ.copy()
        environment["CARGO_TARGET_DIR"] = str(target)
        subprocess.run(command, cwd=ROOT, env=environment, check=True)

        expected = {
            f'{package["name"]}-{package["version"]}.crate' for package in publishable
        }
        actual = {archive.name for archive in (target / "package").glob("*.crate")}
        if actual != expected:
            missing = sorted(expected - actual)
            unexpected = sorted(actual - expected)
            raise SystemExit(
                f"package archive mismatch: missing={missing}, unexpected={unexpected}"
            )

        if args.output_dir:
            args.output_dir.mkdir(parents=True, exist_ok=True)
            for archive_name in sorted(expected):
                shutil.copy2(target / "package" / archive_name, args.output_dir / archive_name)

    print(
        f"PACKAGE_WORKSPACE=PASS archives={len(publishable)} "
        f"excluded={','.join(package['name'] for package in excluded) or '-'}"
    )


if __name__ == "__main__":
    main()
