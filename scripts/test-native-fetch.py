#!/usr/bin/env python3
"""Exercise the immutable FetchContent CMake path against the exact local commit."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run(*args: str, cwd: Path, env: dict[str, str]) -> str:
    process = subprocess.run(
        args,
        cwd=cwd,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if process.returncode:
        raise RuntimeError(
            f"command failed ({process.returncode}): {' '.join(args)}\n{process.stdout}"
        )
    return process.stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-type", choices=("Debug", "Release"), default="Release")
    args = parser.parse_args()
    if run("git", "status", "--porcelain", cwd=ROOT, env=os.environ.copy()).strip():
        raise SystemExit("immutable fetch test requires a clean worktree")
    commit = run("git", "rev-parse", "HEAD", cwd=ROOT, env=os.environ.copy()).strip()
    run(
        "git",
        "cat-file",
        "-e",
        f"{commit}:native/CMakeLists.txt",
        cwd=ROOT,
        env=os.environ.copy(),
    )
    with tempfile.TemporaryDirectory(prefix="axiolid-fetch-") as temporary:
        work = Path(temporary)
        source = work / "consumer"
        source.mkdir()
        shutil.copy2(ROOT / "tests/native/cmake-consumer/main.c", source / "main.c")
        helper = (ROOT / "native/cmake/AxiolidFetch.cmake").as_posix()
        source.joinpath("CMakeLists.txt").write_text(
            "cmake_minimum_required(VERSION 3.24)\n"
            "project(AxiolidFetchConsumer LANGUAGES C)\n"
            f'include("{helper}")\n'
            "axiolid_fetch(\n"
            f'  GIT_REPOSITORY "{ROOT.as_uri()}"\n'
            f'  GIT_COMMIT "{commit}"\n'
            "  LINKAGE SHARED\n)\n"
            "add_executable(fetch-consumer main.c)\n"
            "target_link_libraries(fetch-consumer PRIVATE Axiolid::axiolid)\n"
            "enable_testing()\nadd_test(NAME fetch-consumer COMMAND fetch-consumer)\n"
        )
        build = work / "build"
        env = os.environ.copy()
        run(
            "cmake",
            "-S",
            str(source),
            "-B",
            str(build),
            f"-DCMAKE_BUILD_TYPE={args.build_type}",
            cwd=ROOT,
            env=env,
        )
        run(
            "cmake",
            "--build",
            str(build),
            "--config",
            args.build_type,
            "--parallel",
            "2",
            cwd=ROOT,
            env=env,
        )
        output = run(
            "ctest",
            "--test-dir",
            str(build),
            "-C",
            args.build_type,
            "--output-on-failure",
            cwd=ROOT,
            env=env,
        )
        if "100% tests passed" not in output:
            raise RuntimeError(output)
    print(f"immutable CMake fetch: {commit} {args.build_type}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
