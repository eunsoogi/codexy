"""Mask retained responsibility and branch-reference data from ownership scans."""

from __future__ import annotations

import re

_RETAINED_RESPONSIBILITY = re.compile(
    r"(?i)\b(?:i|we|the caller|the requester)\s+(?:retain|retains|keep|keeps|keeping|remain|remains|stay|stays|continue\s+to)\b(?:(?!\b(?:and|but|while|where|then|so)\b)[^,;.!?:])*?(?:ownership|responsibility|accountability|responsible\s+for|reviewer)\b[^\n,;.!?:]*?(?=\s+(?:and|but|while|where|then|so)\b|[,\n;.!?:]|$)"
)
_COMMIT_METADATA = re.compile(
    r"(?i)\b(?:(?:base|head|parent|current|previous)\s+)?commit\s+[0-9a-f]{7,64}(?=\s+(?:on|from)\s+branch\b|[.,;!?]|$)"
)
_BRANCH_REF = re.compile(r"(?i)\bbranch\s+([^\s,;.!?:]+)")


def mask_non_delegating_data(message: str) -> str:
    masked = _RETAINED_RESPONSIBILITY.sub(
        lambda match: " " * len(match.group()), message
    )
    chars = list(masked)
    for match in _COMMIT_METADATA.finditer(masked):
        start, end = match.span()
        chars[start:end] = [" "] * (end - start)
    for match in _BRANCH_REF.finditer(masked):
        if any(char in match.group(1) for char in "/_.-"):
            start, end = match.span(1)
            chars[start:end] = [" "] * (end - start)
    return "".join(chars)
