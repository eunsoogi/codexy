"""Cargo test-output parsing patterns shared by profile accounting."""

from __future__ import annotations

import re


# Preserve known non-word child-output splices without accepting `... okay`.
RUN_PATTERN = re.compile(
    r"^test (?P<name>.+) \.\.\. (?P<result>ok|FAILED|ignored)(?:done\.|[A-Z].*|[^\sA-Za-z0-9].*)?$"
)
