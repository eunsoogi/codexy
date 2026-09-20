"""Mask retained responsibility and branch-reference data from ownership scans."""

from __future__ import annotations

import re

_CALLER_RETENTION = (
    r"\b(?:i|we|the caller|the requester)\s+"
    r"(?:retain|retains|keep|keeps|keeping|remain|remains|stay|stays|continue\s+to)\b"
)
_CLAUSE_BOUNDARY = r"(?:and|but|while|where|then|so)"
_CLAUSE_LOCAL = r"(?:(?!\b" + _CLAUSE_BOUNDARY + r"\b)[^\n,;.!?:])*?"
_HOLDER_PREPOSITION = r"(?:with|to|for)"
_NON_CALLER_HOLDER = (
    r"(?:you|your|their|them|his|her|its|"
    r"the\s+(?:reviewer|agent|worker|owner|implementer)|"
    r"(?:an?|another|assigned|current|child-owned)\s+"
    r"(?:reviewer|agent|worker|owner|implementer)|\w+['’]s)"
)
_CALLER_POSSESSOR = r"(?:my|our|the\s+caller['’]s|the\s+requester['’]s)"
_NOMINAL_WORD = (
    r"(?:(?!(?:"
    + _CLAUSE_BOUNDARY
    + r"|"
    + _NON_CALLER_HOLDER
    + r"|\w+['’]s)\b)[a-z][\w-]*\s+)"
)
_CALLER_NOMINAL = (
    r"(?:"
    + _CALLER_POSSESSOR
    + r"\s+"
    + _NOMINAL_WORD
    + r"{0,3}"
    + r"|"
    + _NOMINAL_WORD
    + r"{0,4}"
    + r")\b(?:ownership|responsibility|accountability)\b"
)
_RETAINED_RESPONSIBILITY = re.compile(
    r"(?i)(?:"
    + _CALLER_RETENTION
    + r"\s+(?:the\s+)?reviewer\b"
    + r"|"
    + _CALLER_RETENTION
    + r"\s+responsible\s+for\b[^\n,;.!?:]*?"
    + r"(?=\s+"
    + _CLAUSE_BOUNDARY
    + r"\b|[,\n;.!?:]|$)"
    + r"|"
    + _CALLER_RETENTION
    + r"\s+"
    + _CALLER_NOMINAL
    + r"(?!"
    + _CLAUSE_LOCAL
    + r"\s+"
    + _HOLDER_PREPOSITION
    + r"\s+"
    + _NON_CALLER_HOLDER
    + r"\b)"
    + _CLAUSE_LOCAL
    + r"(?=\s+"
    + _CLAUSE_BOUNDARY
    + r"\b|[,\n;.!?:]|$)"
    + r")"
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
