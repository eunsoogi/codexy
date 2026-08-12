"""Markdown body primitives shared by owned GitHub mutation policies."""

from __future__ import annotations

import re


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
                    visible, rest, in_comment = visible + rest[:start], rest[start + 4 :], True
        trimmed = visible.lstrip(" ")
        marker = re.match(r"(`{3,}|~{3,})", trimmed)
        if marker:
            fence = marker.group(1)
        elif trimmed.startswith("## "):
            headings.add(trimmed.strip())
    return headings
