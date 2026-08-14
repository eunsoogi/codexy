"""Cargo test-output parsing patterns shared by profile accounting."""

from __future__ import annotations

import re


# The hosted profiler evidence contains only the direct `okdone.` splice.
RUN_PATTERN = re.compile(
    r"^test (?P<name>.+) \.\.\. (?P<result>ok|FAILED|ignored)(?P<splice>done\.)?$"
)
AMBIGUOUS_RESULT_PATTERN = re.compile(
    r"^test .+ \.\.\. (?:ok|FAILED|ignored)(?:\S.*|\s+.*)$"
)


def accepted_outcome(match: re.Match[str]) -> bool:
    """Accept the evidenced splice only for Cargo's `ok` outcome."""
    return match.group("splice") is None or match.group("result") == "ok"


def accepted_result(line: str) -> re.Match[str] | None:
    match = RUN_PATTERN.match(line)
    return match if match is not None and accepted_outcome(match) else None


def is_ambiguous_result(line: str) -> bool:
    return AMBIGUOUS_RESULT_PATTERN.match(line) is not None
