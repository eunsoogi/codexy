"""Cargo test-output parsing patterns shared by profile accounting."""

from __future__ import annotations

import re


# The hosted profiler evidence contains only the direct `okdone.` splice.
RUN_PATTERN = re.compile(
    r"^test (?P<name>.+) \.\.\. (?P<result>ok|FAILED|ignored)(?P<splice>done\.)?$"
)


def accepted_outcome(match: re.Match[str]) -> bool:
    """Accept the evidenced splice only for Cargo's `ok` outcome."""
    return match.group("splice") is None or match.group("result") == "ok"
