"""Typed GitHub selector normalization for mutation admission."""

from __future__ import annotations

import re
from dataclasses import dataclass


@dataclass(frozen=True)
class PullRequestSelector:
    repository: str | None
    number: int


def pull_request(value: str) -> PullRequestSelector | None:
    if value.isascii() and value.isdigit() and int(value) > 0:
        return PullRequestSelector(None, int(value))
    match = re.fullmatch(r"https://github\.com/([^/\s]+)/([^/\s]+)/pull/([1-9][0-9]*)/?", value)
    if match is None:
        return None
    return PullRequestSelector(f"{match.group(1)}/{match.group(2)}", int(match.group(3)))
