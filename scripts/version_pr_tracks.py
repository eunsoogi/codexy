"""Canonical non-closing governing-issue linkage parsing."""

from __future__ import annotations

import re


TRACKS_CANDIDATE_PATTERN = re.compile(r"(?i)\btracks\b")
TRACKS_DIRECTIVE_PATTERN = re.compile(r"Tracks #([1-9][0-9]*)")


def parse_tracks_issue_number(body: str) -> int:
    """Require exactly one canonical `Tracks` directive in final position."""
    lines = body.splitlines()
    nonempty = [(index, line) for index, line in enumerate(lines) if line]
    directives = [(index, line) for index, line in enumerate(lines) if TRACKS_CANDIDATE_PATTERN.search(line)]
    if len(directives) != 1 or not nonempty or directives[0][0] != nonempty[-1][0]:
        raise ValueError("observed provisional release PR body must end with exactly one canonical Tracks issue linkage")
    match = TRACKS_DIRECTIVE_PATTERN.fullmatch(directives[0][1])
    if match is None:
        raise ValueError("observed provisional release PR body must end with exactly one canonical Tracks issue linkage")
    return int(match[1])
