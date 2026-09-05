"""Shared title predicates for the installed generic GitHub checker."""

from __future__ import annotations

import re


_TYPE = re.compile(r"^[a-z0-9-]+$")
_SCOPE = re.compile(r"^[a-z0-9_/-]+$")
_REFERENCE = re.compile(
    r"(?:^|\s)(?:#[0-9]+|\(#[0-9]+\)|\[#[0-9]+\]|\((?:pr|issue)\s+#[0-9]+\)|(?:pr|issue)\s+#[0-9]+)$",
    re.IGNORECASE,
)


def _prefix(value: str) -> bool:
    value = value.removesuffix("!")
    if "(" not in value or not value.endswith(")"):
        return False
    commit_type, scope = value.split("(", 1)
    return (
        ")" not in scope[:-1]
        and bool(_TYPE.fullmatch(commit_type))
        and bool(_SCOPE.fullmatch(scope[:-1]))
    )


def _invalid_character(value: str) -> bool:
    return any(
        ord(char) < 32 or ord(char) == 127 or ord(char) in {0x85, 0x2028, 0x2029}
        for char in value
    )


def _terminal_reference(value: str) -> bool:
    value = value.strip()
    while value.endswith((".", ",")):
        value = value[:-1].rstrip()
    return bool(_REFERENCE.search(value))


def pr_title(value: object) -> bool:
    if not isinstance(value, str) or _invalid_character(value) or ": " not in value:
        return False
    prefix, summary = value.split(": ", 1)
    return (
        bool(summary.strip()) and _prefix(prefix) and not _terminal_reference(summary)
    )


def _category_prefix(value: str) -> tuple[int, bool, bool] | None:
    index = 0
    while (
        index < len(value)
        and value[index].isascii()
        and (value[index].isalnum() or value[index] == "-")
    ):
        index += 1
    if not _TYPE.fullmatch(value[:index].lower()):
        return None
    type_end = index
    next_index = _spaces(value, type_end)
    scoped = False
    index = type_end
    if next_index < len(value) and value[next_index] == "(":
        scoped = True
        index = _spaces(value, next_index + 1)
        start = index
        while (
            index < len(value)
            and value[index].isascii()
            and (value[index].isalnum() or value[index] in "-_/")
        ):
            index += 1
        if start == index or not _SCOPE.fullmatch(value[start:index].lower()):
            return None
        index = _spaces(value, index)
        if index == len(value) or value[index] != ")":
            return None
        index += 1
    breaking_index = _spaces(value, index)
    breaking = breaking_index < len(value) and value[breaking_index] == "!"
    if breaking:
        index = _spaces(value, breaking_index + 1)
    return index, scoped, breaking


def _spaces(value: str, index: int) -> int:
    while index < len(value) and value[index] in " \t":
        index += 1
    return index


def _dash_separator(value: str) -> bool:
    return bool(value) and value[0] in "-–—" and (len(value) == 1 or value[1] in " \t")


def _issue_category(value: str) -> bool:
    if value.startswith("[") and "]" in value:
        end = value.index("]")
        inner = value[1:end]
        parsed = _category_prefix(inner)
        if parsed is not None and parsed[0] == len(inner):
            return True
    parsed = _category_prefix(value)
    if parsed is None:
        return False
    index, scoped, breaking = parsed
    rest = value[index:]
    if scoped or breaking:
        return not rest or rest[:1] in " \t" or rest[:1] in ":：-–—"
    if not rest or not rest.strip(" \t"):
        return True
    trimmed = rest.lstrip(" \t")
    return trimmed[:1] in ":：" or _dash_separator(trimmed)


def issue_title(value: object) -> bool:
    if not isinstance(value, str) or not value or not value[0].isascii():
        return False
    if not value[0].isupper() or _invalid_character(value):
        return False
    return not _issue_category(value)
