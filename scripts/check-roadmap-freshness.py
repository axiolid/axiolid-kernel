#!/usr/bin/env python3
"""Keep docs/ROADMAP.md from drifting back into a status page.

The roadmap explains *ordering and reasoning*. The moment it starts carrying
per-item status, it starts going stale — the exact failure this gate exists to
prevent.

Checks:
  1. No task checkboxes. A checklist here duplicates the project board.
  2. No progress words ("now:", "next:", "in progress", "completed") used as
     section headings, which imply a status this page cannot keep current.
  3. Every milestone that exists on GitHub is mentioned, so a new milestone
     cannot be invisible here. (Skipped without network/gh.)
  4. The pointer block to the board/milestones/capabilities is intact.

Run: python3 scripts/check-roadmap-freshness.py
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROADMAP = Path(__file__).resolve().parent.parent / "docs" / "ROADMAP.md"

# Headings that assert a state this page cannot keep current.
STALE_HEADINGS = re.compile(
    r"^#{2,3}\s+.*\b(now|next|then|in progress|completed|current status)\b",
    re.IGNORECASE | re.MULTILINE,
)
CHECKBOX = re.compile(r"^\s*[-*]\s+\[[ xX]\]", re.MULTILINE)
REQUIRED_POINTERS = (
    "orgs/axiolid/projects/1",
    "axiolid/kernel/milestones",
    "capabilities",
)


def milestones_on_github() -> list[str] | None:
    """Milestone titles, or None when GitHub is unreachable."""
    try:
        out = subprocess.run(
            ["gh", "api", "repos/axiolid/kernel/milestones", "--paginate"],
            capture_output=True, text=True, timeout=30,
        )
        if out.returncode != 0:
            return None
        return [m["title"] for m in json.loads(out.stdout)]
    except Exception:
        return None


def main() -> int:
    if not ROADMAP.exists():
        print(f"roadmap: {ROADMAP} not found")
        return 1

    text = ROADMAP.read_text(encoding="utf-8")
    problems: list[str] = []

    for match in CHECKBOX.finditer(text):
        line = text[: match.start()].count("\n") + 1
        problems.append(
            f"line {line}: task checkbox — per-item status belongs on the "
            f"project board, not here"
        )

    for match in STALE_HEADINGS.finditer(text):
        line = text[: match.start()].count("\n") + 1
        heading = match.group(0).strip()
        problems.append(
            f"line {line}: heading {heading!r} asserts progress state this "
            f"page cannot keep current"
        )

    for pointer in REQUIRED_POINTERS:
        if pointer not in text:
            problems.append(
                f"missing pointer to {pointer!r} — readers must be sent to the "
                f"live source"
            )

    titles = milestones_on_github()
    if titles is None:
        print("roadmap: skipping milestone coverage (gh unavailable)")
    else:
        for title in titles:
            # Match on the version prefix; the prose after the dash may differ.
            key = title.split("—")[0].strip()
            if key and key.lower() not in text.lower():
                problems.append(
                    f"milestone {title!r} exists on GitHub but is not "
                    f"explained here"
                )

    if problems:
        print(f"roadmap: {len(problems)} problem(s):")
        for problem in problems:
            print(f"- {problem}")
        return 1

    print("roadmap freshness: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
