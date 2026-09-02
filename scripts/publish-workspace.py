#!/usr/bin/env python3
"""Publish Axiolid crates to crates.io in stable dependency order.

`cargo publish --workspace` is unstable in Cargo 1.88. This script derives an
acyclic order from every internal normal, build, and dev dependency, then
publishes one package at a time. Existing immutable versions are skipped only
when their crates.io checksum matches the exact unpatched local package, so a
workflow interrupted by registry propagation or rate limits can be rerun.

The default mode is a side-effect-free plan. Actual publication requires both
`--execute` and Cargo's `CARGO_REGISTRY_TOKEN` environment variable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARGO = os.environ.get("CARGO", "cargo")
USER_AGENT = "axiolid-release-workflow/1.0"


def metadata() -> dict:
    result = subprocess.run(
        [CARGO, "metadata", "--format-version", "1", "--no-deps", "--locked"],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    return json.loads(result.stdout)


def publish_plan(data: dict) -> list[dict]:
    members = set(data["workspace_members"])
    packages = {
        package["name"]: package
        for package in data["packages"]
        if package["id"] in members and package.get("publish") != []
    }
    by_path = {
        str(Path(package["manifest_path"]).parent.resolve()): package["name"]
        for package in packages.values()
    }
    dependencies: dict[str, set[str]] = {}
    for name, package in packages.items():
        dependencies[name] = {
            by_path[str(Path(dependency["path"]).resolve())]
            for dependency in package["dependencies"]
            if dependency.get("path")
            and str(Path(dependency["path"]).resolve()) in by_path
        }

    order: list[str] = []
    remaining = {name: set(required) for name, required in dependencies.items()}
    while remaining:
        ready = sorted(name for name, required in remaining.items() if not required)
        if not ready:
            cycle = ", ".join(
                f"{name}->[{', '.join(sorted(required))}]"
                for name, required in sorted(remaining.items())
            )
            raise SystemExit(f"internal publication dependency cycle: {cycle}")
        for name in ready:
            order.append(name)
            del remaining[name]
        published = set(ready)
        for required in remaining.values():
            required.difference_update(published)

    return [packages[name] for name in order]


def registry_checksum(name: str, version: str) -> str | None | bool:
    """Return checksum, False when absent, or None on a transient lookup failure."""
    encoded_name = urllib.parse.quote(name, safe="")
    encoded_version = urllib.parse.quote(version, safe="")
    request = urllib.request.Request(
        f"https://crates.io/api/v1/crates/{encoded_name}/{encoded_version}",
        headers={"User-Agent": USER_AGENT},
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = json.load(response)
            checksum = payload.get("version", {}).get("checksum")
            if not isinstance(checksum, str) or len(checksum) != 64:
                raise SystemExit(f"crates.io returned no valid checksum for {name} {version}")
            return checksum
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return False
        print(f"warning: crates.io lookup for {name} returned HTTP {error.code}", file=sys.stderr)
        return None
    except (OSError, TimeoutError) as error:
        print(f"warning: crates.io lookup for {name} failed: {error}", file=sys.stderr)
        return None


def archive_checksum(archive: Path) -> str:
    digest = hashlib.sha256()
    with archive.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require_matching_checksum(name: str, version: str, local_checksum: str) -> bool:
    remote_checksum = registry_checksum(name, version)
    if remote_checksum is False or remote_checksum is None:
        return False
    if remote_checksum != local_checksum:
        raise SystemExit(
            f"immutable version collision for {name} {version}: "
            f"local={local_checksum} crates.io={remote_checksum}"
        )
    return True


def wait_for_matching_checksum(
    name: str, version: str, local_checksum: str, args: argparse.Namespace
) -> None:
    for attempt in range(1, args.max_attempts + 1):
        if require_matching_checksum(name, version, local_checksum):
            return
        if attempt == args.max_attempts:
            break
        print(
            f"WAIT_CHECKSUM {name} attempt={attempt + 1} wait={args.dependency_wait}s",
            flush=True,
        )
        time.sleep(args.dependency_wait)
    raise SystemExit(
        f"crates.io did not expose {name} {version} with checksum {local_checksum} "
        f"after {args.max_attempts} attempts"
    )


def retry_delay(output: str, args: argparse.Namespace) -> tuple[int, str] | None:
    lowered = output.lower()
    if "too many requests" in lowered or "429" in lowered:
        return args.rate_limit_wait, "crates.io rate limit"
    if "no matching package named" in lowered or "failed to select a version" in lowered:
        return args.dependency_wait, "registry dependency propagation"
    return None


def prepare_archive(
    package: dict,
    target: Path,
    environment: dict[str, str],
    args: argparse.Namespace,
) -> Path:
    name = package["name"]
    version = package["version"]
    archive = target / "package" / f"{name}-{version}.crate"
    for attempt in range(1, args.max_attempts + 1):
        archive.unlink(missing_ok=True)
        result = subprocess.run(
            [CARGO, "package", "-p", name, "--locked", "--quiet"],
            cwd=ROOT,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        if result.stdout:
            print(result.stdout, end="", flush=True)
        if result.returncode == 0:
            if not archive.is_file():
                raise SystemExit(f"cargo package produced no archive for {name} {version}")
            return archive

        retry = retry_delay(result.stdout, args)
        if retry is None:
            raise SystemExit(f"cargo package failed permanently for {name} {version}")
        if attempt == args.max_attempts:
            raise SystemExit(
                f"cargo package exhausted {args.max_attempts} attempts for {name} {version}"
            )
        wait, reason = retry
        print(
            f"RETRY_PACKAGE {name} attempt={attempt + 1} wait={wait}s reason={reason}",
            flush=True,
        )
        time.sleep(wait)
    raise AssertionError("unreachable package retry loop")


def publish(
    package: dict,
    target: Path,
    environment: dict[str, str],
    args: argparse.Namespace,
) -> None:
    name = package["name"]
    version = package["version"]
    archive = prepare_archive(package, target, environment, args)
    local_checksum = archive_checksum(archive)
    if require_matching_checksum(name, version, local_checksum):
        print(f"SKIP {name} {version}: published checksum matches {local_checksum}", flush=True)
        return

    for attempt in range(1, args.max_attempts + 1):
        result = subprocess.run(
            [CARGO, "publish", "-p", name, "--locked"],
            cwd=ROOT,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        if result.stdout:
            print(result.stdout, end="", flush=True)
        if result.returncode == 0:
            wait_for_matching_checksum(name, version, local_checksum, args)
            print(f"PUBLISHED {name} {version} checksum={local_checksum}", flush=True)
            return

        output = result.stdout.lower()
        if "already uploaded" in output or "already exists" in output:
            wait_for_matching_checksum(name, version, local_checksum, args)
            print(f"SKIP {name} {version}: published checksum matches", flush=True)
            return

        retry = retry_delay(result.stdout, args)
        if retry is None:
            raise SystemExit(f"cargo publish failed permanently for {name} {version}")
        if attempt == args.max_attempts:
            raise SystemExit(
                f"cargo publish exhausted {args.max_attempts} attempts for {name} {version}"
            )
        wait, reason = retry
        print(
            f"RETRY_PUBLISH {name} attempt={attempt + 1} wait={wait}s reason={reason}",
            flush=True,
        )
        time.sleep(wait)
    raise AssertionError("unreachable publish retry loop")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--execute", action="store_true", help="perform irreversible uploads")
    parser.add_argument("--max-attempts", type=int, default=30)
    parser.add_argument("--dependency-wait", type=int, default=30)
    parser.add_argument("--rate-limit-wait", type=int, default=610)
    args = parser.parse_args()
    if args.max_attempts < 1 or args.dependency_wait < 0 or args.rate_limit_wait < 0:
        parser.error("attempts must be positive and wait values must be non-negative")

    plan = publish_plan(metadata())
    print(f"PUBLISH_PLAN={len(plan)}")
    for index, package in enumerate(plan, 1):
        print(f"{index:02d} {package['name']} {package['version']}")

    if not args.execute:
        print("PLAN_ONLY: pass --execute with CARGO_REGISTRY_TOKEN to publish")
        return
    if not os.environ.get("CARGO_REGISTRY_TOKEN"):
        raise SystemExit("CARGO_REGISTRY_TOKEN is required with --execute")

    with tempfile.TemporaryDirectory(prefix="axiolid-publish-") as temporary:
        target = Path(temporary) / "target"
        environment = os.environ.copy()
        environment["CARGO_TARGET_DIR"] = str(target)
        for package in plan:
            publish(package, target, environment, args)
    print(f"PUBLISH_WORKSPACE=PASS crates={len(plan)}")


if __name__ == "__main__":
    main()
