#!/usr/bin/env python3
"""Release-script regression tests; network-free and side-effect-free."""

from __future__ import annotations

import hashlib
import importlib.util
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]


def load_script(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / filename)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {filename}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


publish = load_script("publish_workspace", "publish-workspace.py")


class PublishWorkspaceTests(unittest.TestCase):
    def test_plan_orders_every_internal_dependency_before_its_consumer(self) -> None:
        data = publish.metadata()
        plan = publish.publish_plan(data)
        positions = {package["name"]: index for index, package in enumerate(plan)}
        by_path = {
            str(Path(package["manifest_path"]).parent.resolve()): package["name"]
            for package in plan
        }
        self.assertEqual(len(plan), 31)
        for package in plan:
            for dependency in package["dependencies"]:
                dependency_path = dependency.get("path")
                if dependency_path and str(Path(dependency_path).resolve()) in by_path:
                    dependency_name = by_path[str(Path(dependency_path).resolve())]
                    self.assertLess(
                        positions[dependency_name],
                        positions[package["name"]],
                        f"{dependency_name} must publish before {package['name']}",
                    )

    def test_matching_registry_checksum_is_idempotent(self) -> None:
        checksum = "a" * 64
        with mock.patch.object(publish, "registry_checksum", return_value=checksum):
            self.assertTrue(publish.require_matching_checksum("crate", "1.0.0", checksum))

    def test_version_collision_fails_closed(self) -> None:
        with mock.patch.object(publish, "registry_checksum", return_value="b" * 64):
            with self.assertRaisesRegex(SystemExit, "immutable version collision"):
                publish.require_matching_checksum("crate", "1.0.0", "a" * 64)

    def test_absent_registry_version_is_not_treated_as_published(self) -> None:
        with mock.patch.object(publish, "registry_checksum", return_value=False):
            self.assertFalse(publish.require_matching_checksum("crate", "1.0.0", "a" * 64))

    def test_archive_checksum_hashes_exact_bytes(self) -> None:
        payload = b"normalized crate archive\x00"
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "crate-1.0.0.crate"
            archive.write_bytes(payload)
            self.assertEqual(publish.archive_checksum(archive), hashlib.sha256(payload).hexdigest())

    def test_prepare_archive_uses_unpatched_locked_package(self) -> None:
        package = {"name": "leaf", "version": "0.1.0"}
        args = SimpleNamespace(max_attempts=1, dependency_wait=0, rate_limit_wait=0)
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / "target"

            def fake_run(command, **kwargs):
                self.assertEqual(
                    command,
                    [publish.CARGO, "package", "-p", "leaf", "--locked", "--quiet"],
                )
                self.assertNotIn("--config", command)
                self.assertEqual(kwargs["env"]["CARGO_TARGET_DIR"], str(target))
                archive = target / "package" / "leaf-0.1.0.crate"
                archive.parent.mkdir(parents=True)
                archive.write_bytes(b"crate")
                return SimpleNamespace(returncode=0, stdout="")

            environment = {"CARGO_TARGET_DIR": str(target)}
            with mock.patch.object(publish.subprocess, "run", side_effect=fake_run):
                archive = publish.prepare_archive(package, target, environment, args)
            self.assertEqual(archive.read_bytes(), b"crate")

    def test_prepare_archive_fails_closed_on_permanent_error(self) -> None:
        package = {"name": "leaf", "version": "0.1.0"}
        args = SimpleNamespace(max_attempts=1, dependency_wait=0, rate_limit_wait=0)
        result = SimpleNamespace(returncode=101, stdout="")
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / "target"
            with mock.patch.object(publish.subprocess, "run", return_value=result):
                with self.assertRaisesRegex(SystemExit, "failed permanently"):
                    publish.prepare_archive(package, target, {}, args)


if __name__ == "__main__":
    unittest.main()
