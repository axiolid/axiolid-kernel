#!/usr/bin/env python3
"""Prove source-build and packaged CMake consumers have equivalent behavior."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONSUMER = ROOT / "tests/native/cmake-consumer"
C_MARKER = re.compile(r"axiolid native consumer: (\d+\.\d+)")
CPP_MARKER = re.compile(r"axiolid native C\+\+ consumer: (\d+\.\d+)")


def run(args: list[str], *, env: dict[str, str] | None = None) -> str:
    process = subprocess.run(
        args,
        cwd=ROOT,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if process.returncode:
        print(process.stdout, file=sys.stderr)
        raise RuntimeError(f"command failed ({process.returncode}): {' '.join(args)}")
    return process.stdout


def configure_build_test(
    build: Path, build_type: str, linkage: str, extra: list[str], env: dict[str, str]
) -> str:
    run(
        [
            "cmake",
            "-S",
            str(CONSUMER),
            "-B",
            str(build),
            f"-DCMAKE_BUILD_TYPE={build_type}",
            f"-DAXIOLID_LINKAGE={linkage}",
            *extra,
        ],
        env=env,
    )
    run(
        ["cmake", "--build", str(build), "--config", build_type, "--parallel", "2"],
        env=env,
    )
    output = run(
        [
            "ctest",
            "--test-dir",
            str(build),
            "-C",
            build_type,
            "-V",
            "--output-on-failure",
        ],
        env=env,
    )
    c_match = C_MARKER.search(output)
    cpp_match = CPP_MARKER.search(output)
    if not c_match or not cpp_match:
        raise RuntimeError("native C and C++ consumers did not both emit ABI markers")
    if c_match.group(1) != cpp_match.group(1):
        raise RuntimeError("native C and C++ consumers reported different ABI versions")
    return c_match.group(1)


def assert_embedded_build_type_unchanged(
    work: Path, cargo_target: Path, env: dict[str, str]
) -> None:
    source = work / "embedding"
    source.mkdir()
    (source / "CMakeLists.txt").write_text("""cmake_minimum_required(VERSION 3.24)
project(Embedding C)
set(before "${CMAKE_BUILD_TYPE}")
add_subdirectory("${AXIOLID_ROOT}/native" axiolid)
if(NOT "${CMAKE_BUILD_TYPE}" STREQUAL "${before}")
  message(FATAL_ERROR "Axiolid changed the consumer CMAKE_BUILD_TYPE")
endif()
""")
    run(
        [
            "cmake",
            "-S",
            str(source),
            "-B",
            str(work / "embedding-build"),
            f"-DAXIOLID_ROOT={ROOT}",
            f"-DAXIOLID_CARGO_TARGET_DIR={cargo_target}",
        ],
        env=env,
    )


def install_source_package(
    build: Path,
    prefix: Path,
    build_type: str,
    cargo_target: Path,
    env: dict[str, str],
) -> None:
    run(
        [
            "cmake",
            "-S",
            str(ROOT / "native"),
            "-B",
            str(build),
            f"-DCMAKE_BUILD_TYPE={build_type}",
            f"-DAXIOLID_CARGO_TARGET_DIR={cargo_target}",
        ],
        env=env,
    )
    run(
        ["cmake", "--build", str(build), "--config", build_type, "--parallel", "2"],
        env=env,
    )
    run(
        [
            "cmake",
            "--install",
            str(build),
            "--config",
            build_type,
            "--prefix",
            str(prefix),
        ],
        env=env,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-type", choices=("Debug", "Release"), default="Release")
    parser.add_argument("--linkage", choices=("SHARED", "STATIC"), default="SHARED")
    args = parser.parse_args()
    profile = "debug" if args.build_type == "Debug" else "release"
    try:
        with tempfile.TemporaryDirectory(prefix="axiolid-cmake-") as temporary:
            work = Path(temporary)
            env = os.environ.copy()
            cargo_target = Path(
                env.get("CARGO_TARGET_DIR", work / "cargo-target")
            ).resolve()
            env["CARGO_TARGET_DIR"] = str(cargo_target)
            assert_embedded_build_type_unchanged(work, cargo_target, env)
            source = configure_build_test(
                work / "source",
                args.build_type,
                args.linkage,
                [
                    f"-DAXIOLID_SOURCE_TREE={ROOT}",
                    f"-DAXIOLID_CARGO_TARGET_DIR={cargo_target}",
                ],
                env,
            )
            install_prefix = work / "installed"
            install_source_package(
                work / "install-build",
                install_prefix,
                args.build_type,
                cargo_target,
                env,
            )
            installed = configure_build_test(
                work / "installed-consumer",
                args.build_type,
                args.linkage,
                [f"-DCMAKE_PREFIX_PATH={install_prefix}"],
                env,
            )
            package_output = work / "packages"
            package_log = run(
                [
                    sys.executable,
                    str(ROOT / "scripts/package-native.py"),
                    "--profile",
                    profile,
                    "--output-dir",
                    str(package_output),
                    "--allow-dirty",
                ],
                env=env,
            )
            archive = Path(package_log.strip().splitlines()[-1])
            extract_to = work / "extracted"
            verify_log = run(
                [
                    sys.executable,
                    str(ROOT / "scripts/verify-native-package.py"),
                    str(archive),
                    "--allow-dirty",
                    "--extract-to",
                    str(extract_to),
                ],
                env=env,
            )
            package_root = Path(verify_log.strip().splitlines()[-1])
            packaged = configure_build_test(
                work / "package",
                args.build_type,
                args.linkage,
                [f"-DCMAKE_PREFIX_PATH={package_root}"],
                env,
            )
            if source != installed or source != packaged or source != "0.4":
                raise RuntimeError(
                    "source/install/archive behavior differs: "
                    f"{source!r}, {installed!r}, {packaged!r}"
                )
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"native CMake test failed: {error}", file=sys.stderr)
        return 1
    print(f"native CMake equivalence: {args.build_type} {args.linkage} ABI {source}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
