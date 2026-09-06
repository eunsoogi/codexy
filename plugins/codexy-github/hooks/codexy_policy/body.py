"""Markdown body primitives shared by owned GitHub mutation policies."""

from __future__ import annotations

import re

PR_REQUIRED_SECTIONS = frozenset(
    {
        "## Summary",
        "## Rationale",
        "## Changed Areas",
        "## Verification",
        "## Evidence",
        "## Not Run",
        "## Follow-ups",
    }
)
PR_CLOSING = re.compile(
    r"\b(?:close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved)\s+#([1-9][0-9]*)\b",
    re.IGNORECASE,
)


def has_sections(value: object, required: set[str]) -> bool:
    return isinstance(value, str) and required.issubset(visible_headings(value))


def visible_headings(value: str) -> set[str]:
    headings: set[str] = set()
    fence: str | None = None
    in_comment = False
    for raw in value.splitlines():
        if fence is not None:
            if re.fullmatch(rf"{re.escape(fence)}[ \t]*", raw.lstrip(" ")):
                fence = None
            continue
        if raw.startswith(("    ", "\t")):
            continue
        visible, rest = "", raw
        while rest:
            if in_comment:
                end = rest.find("-->")
                if end < 0:
                    rest = ""
                else:
                    rest, in_comment = rest[end + 3 :], False
            else:
                start = rest.find("<!--")
                if start < 0:
                    visible += rest
                    rest = ""
                else:
                    visible, rest, in_comment = (
                        visible + rest[:start],
                        rest[start + 4 :],
                        True,
                    )
        trimmed = visible.lstrip(" ")
        marker = re.match(r"(`{3,}|~{3,})", trimmed)
        if marker:
            fence = marker.group(1)
        elif trimmed.startswith("## "):
            headings.add(trimmed.strip())
    return headings


def valid_pull_request_body(value: object, issue: int | None = None) -> bool:
    if not has_sections(value, PR_REQUIRED_SECTIONS):
        return False
    assert isinstance(value, str)
    references = [int(number) for number in PR_CLOSING.findall(value)]
    final = next((line for line in reversed(value.splitlines()) if line.strip()), "")
    return (
        len(references) == 1
        and final == f"Fixes #{references[0]}"
        and (issue is None or references[0] == issue)
    )


def valid_pull_request_update(fields: dict[str, object]) -> bool:
    return (
        bool(fields)
        and set(fields) <= {"title", "body", "base", "maintainer_can_modify"}
        and ("body" not in fields or valid_pull_request_body(fields["body"]))
    )
