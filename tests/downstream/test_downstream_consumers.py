from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "downstream_probes", ROOT / "scripts/test-downstream-consumers.py"
)
assert SPEC and SPEC.loader
probes = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(probes)


class DownstreamProbeTests(unittest.TestCase):
    def test_manifest_is_version_and_revision_bound_without_paths(self) -> None:
        source = """\
[package]
name = "sample"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[dependencies]
axiolid-linear = { path = "../../../crates/linear", default-features = false }
axiolid = { path = "../../../crates/axiolid", features = ["application"] }
"""
        rendered = probes.render_manifest(
            source,
            versions={"axiolid-linear": "0.1.0", "axiolid": "0.1.0"},
            allowed={"axiolid-linear", "axiolid"},
            source_url="file:///tmp/release.git",
            revision="1" * 40,
        )
        self.assertNotIn("path", rendered)
        parsed = tomllib.loads(rendered)
        self.assertEqual(parsed["dependencies"]["axiolid"]["version"], "=0.1.0")
        self.assertEqual(parsed["dependencies"]["axiolid"]["rev"], "1" * 40)
        self.assertEqual(parsed["dependencies"]["axiolid"]["features"], ["application"])
        self.assertFalse(parsed["dependencies"]["axiolid-linear"]["default-features"])

    def test_manifest_rejects_unpublished_package_and_bad_revision(self) -> None:
        source = """\
[package]
name = "sample"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
private-kernel = { path = "../../../private" }
"""
        with self.assertRaisesRegex(ValueError, "unpublished"):
            probes.render_manifest(
                source,
                versions={"private-kernel": "0.1.0"},
                allowed=set(),
                source_url="file:///tmp/release.git",
                revision="1" * 40,
            )
        with self.assertRaisesRegex(ValueError, "40 lowercase hexadecimal"):
            probes.render_manifest(
                source.replace("private-kernel", "axiolid-core"),
                versions={"axiolid-core": "0.1.0"},
                allowed={"axiolid-core"},
                source_url="file:///tmp/release.git",
                revision="main",
            )

    def test_native_probe_policy_rejects_unversioned_symbols(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            probe = Path(temporary)
            (probe / "main.c").write_text(
                "axiolid_v0_4_version(&version);\n", encoding="utf-8"
            )
            probes.verify_native_source_policy(probe)
            (probe / "main.c").write_text(
                "axiolid_version(&version);\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(ValueError, "unversioned ABI"):
                probes.verify_native_source_policy(probe)


if __name__ == "__main__":
    unittest.main()
