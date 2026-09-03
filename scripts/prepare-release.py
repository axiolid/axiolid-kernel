#!/usr/bin/env python3
"""Bump the workspace version and roll docs/CHANGELOG.md for a release.

This is the version/changelog half of issue #10 release automation; the
publish half (dependency order, byte-identical archives, dry-run
reproduction) already exists in publish-workspace.py and verify-packages.py.

Usage:
    python3 scripts/prepare-release.py --release <X.Y.Z> [--check]

--check (default) validates the requested bump and changelog shape without
writing anything. Pass --write to apply the bump and roll the changelog.
"""

from __future__ import annotations

import argparse
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARGO_TOML = ROOT / "Cargo.toml"
CHANGELOG = ROOT / "docs" / "CHANGELOG.md"

SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")
WORKSPACE_VERSION_RE = re.compile(r"(?m)^version = \"(\d+\.\d+\.\d+)\"$")
UNRELEASED_HEADING = "## [Unreleased]"


def current_version() -> str:
    text = CARGO_TOML.read_text(encoding="utf-8")
    match = WORKSPACE_VERSION_RE.search(text)
    if match is None:
        raise SystemExit(f"{CARGO_TOML}: no [workspace.package] version found")
    return match.group(1)


def parse_semver(value: str) -> tuple[int, int, int]:
    match = SEMVER_RE.match(value)
    if match is None:
        raise SystemExit(f"not a valid semantic version: {value!r}")
    return tuple(int(part) for part in match.groups())  # type: ignore[return-value]


def require_forward_bump(current: str, requested: str) -> None:
    """A release version must be strictly greater than the current one.

    This is the guardrail a manual release is most likely to skip under
    time pressure: re-releasing the same version, or a backward bump from
    a typo, both produce an immutable crates.io collision or a confusing
    changelog. Fail before either can happen.
    """
    if parse_semver(requested) <= parse_semver(current):
        raise SystemExit(
            f"requested release {requested} must be strictly greater than "
            f"the current workspace version {current}"
        )


def changelog_unreleased_body() -> tuple[str, str, str]:
    """Split the changelog into (prefix, unreleased body, suffix after the next heading).

    A changelog with no released heading yet (the first release) is valid:
    the Unreleased section simply runs to end of file, and the suffix is empty.
    """
    text = CHANGELOG.read_text(encoding="utf-8")
    start = text.find(UNRELEASED_HEADING)
    if start == -1:
        raise SystemExit(f"{CHANGELOG}: missing {UNRELEASED_HEADING!r} heading")
    body_start = start + len(UNRELEASED_HEADING)
    next_heading = re.search(r"(?m)^## \[", text[body_start:])
    body_end = body_start + next_heading.start() if next_heading else len(text)
    return text[:start], text[body_start:body_end], text[body_end:]


def require_nonempty_unreleased(body: str) -> None:
    """An empty Unreleased section means nothing to release, or a
    forgotten changelog entry. Either way, do not roll it silently."""
    if not any(line.strip().startswith("- ") for line in body.splitlines()):
        raise SystemExit(
            f"{CHANGELOG}: {UNRELEASED_HEADING} has no '- ' entries; "
            "nothing to release, or an entry was forgotten"
        )


def rolled_changelog(prefix: str, body: str, suffix: str, version: str, today: str) -> str:
    """Move the Unreleased body under a new dated version heading and
    restore an empty Unreleased section above it, Keep-a-Changelog style."""
    dated_heading = f"## [{version}] - {today}"
    return f"{prefix}{UNRELEASED_HEADING}\n\n{dated_heading}{body}{suffix}"


def bumped_workspace_toml(version: str, current: str) -> str:
    """Bump `[workspace.package].version` and every internal path-dependency
    `version = "<current>"` requirement in `[workspace.dependencies]`.

    Internal path dependencies pin an explicit minimum-version requirement
    (Cargo does not infer it from the path). Cargo\'s default caret semantics
    on a 0.x version are strict: `version = "0.1.0"` means `^0.1.0`, which
    excludes `0.2.0`. Bumping only `[workspace.package].version` would leave
    every internal dependency unsatisfiable by the crate it points to the
    moment that crate actually publishes at the new version.
    """
    text = CARGO_TOML.read_text(encoding="utf-8")
    replaced, count = WORKSPACE_VERSION_RE.subn(f'version = "{version}"', text, count=1)
    if count != 1:
        raise SystemExit(f"{CARGO_TOML}: expected exactly one workspace version line")

    internal_dependency_re = re.compile(
        r'(?m)^(axiolid[\w-]* = \{[^\n}]*?version = ")' + re.escape(current) + r'("[^\n}]*\})'
    )
    replaced, dependency_count = internal_dependency_re.subn(
        lambda match: f"{match.group(1)}{version}{match.group(2)}", replaced
    )
    if dependency_count == 0:
        raise SystemExit(
            f"{CARGO_TOML}: no internal axiolid-* dependency pinned to {current!r}; "
            "the dependency-version regex may be stale"
        )
    return replaced


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", required=True, help="target version, e.g. 0.2.0")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", default=True, help="validate only (default)")
    mode.add_argument("--write", action="store_true", help="apply the bump and changelog roll")
    args = parser.parse_args()

    current = current_version()
    require_forward_bump(current, args.release)

    prefix, body, suffix = changelog_unreleased_body()
    require_nonempty_unreleased(body)

    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    new_changelog = rolled_changelog(prefix, body, suffix, args.release, today)
    new_toml = bumped_workspace_toml(args.release, current)

    print(f"RELEASE_CHECK current={current} requested={args.release} date={today}")
    print(f"RELEASE_CHECK unreleased_entries={sum(1 for line in body.splitlines() if line.strip().startswith('- '))}")

    if not args.write:
        print("RELEASE_PREPARE=CHECK_ONLY (pass --write to apply)")
        return 0

    CARGO_TOML.write_text(new_toml, encoding="utf-8")
    CHANGELOG.write_text(new_changelog, encoding="utf-8")
    print(f"RELEASE_PREPARE=WRITE version={args.release}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
