"""Canonical non-closing governing-issue linkage parsing."""

from __future__ import annotations

import re


TRACKS_DIRECTIVE_PATTERN = re.compile(r"Tracks #([1-9][0-9]*)")


def parse_tracks_issue_number(body: str) -> int:
    """Require one canonical `Tracks` directive in final position."""
    nonempty = [line for line in body.splitlines() if line.strip()]
    match = TRACKS_DIRECTIVE_PATTERN.fullmatch(nonempty[-1]) if nonempty else None
    if match is None:
        raise ValueError(
            "observed provisional release PR body must end with exactly one canonical Tracks issue linkage"
        )
    return int(match[1])
