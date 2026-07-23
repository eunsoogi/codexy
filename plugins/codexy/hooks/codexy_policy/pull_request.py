"""Typed PR create/update admission contract shared by tool and CLI policies."""

from __future__ import annotations

import re
from typing import Any

from .body import has_sections
from .merge import positive_int
from .titles import pr_title

REQUIRED_SECTIONS = {
    "## Summary", "## Rationale", "## Changed Areas", "## Verification", "## Evidence", "## Not Run", "## Follow-ups",
}
CLOSING = re.compile(r"\b(?:close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved)\s+#([1-9][0-9]*)\b", re.IGNORECASE)


def create(data: dict[str, Any]) -> bool:
    issue = data.get("issue")
    return _valid(data.get("title"), data.get("body"), issue if positive_int(issue) else None) and (issue is None or positive_int(issue))


def update(data: dict[str, Any]) -> bool:
    if not positive_int(data.get("pr_number")):
        return False
    if "title" in data and not pr_title(data["title"]):
        return False
    return "body" not in data or _body(data["body"], None)


def shell_create(title: object, body: object) -> bool:
    return _valid(title, body, None)


def shell_update(number: object, title: object | None, body: object | None, body_present: bool) -> bool:
    data: dict[str, Any] = {"pr_number": number}
    if title is not None:
        data["title"] = title
    if body_present:
        data["body"] = body
    return update(data)


def _valid(title: object, body: object, issue: int | None) -> bool:
    return pr_title(title) and _body(body, issue)


def _body(value: object, issue: int | None) -> bool:
    if not has_sections(value, REQUIRED_SECTIONS):
        return False
    assert isinstance(value, str)
    references = [int(number) for number in CLOSING.findall(value)]
    final = next((line for line in reversed(value.splitlines()) if line.strip()), "")
    if issue is not None:
        return references == [issue] and final == f"Fixes #{issue}"
    return not references or (len(references) == 1 and final == f"Fixes #{references[0]}")
