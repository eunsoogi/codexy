"""Literal GitHub CLI squash-subject inspection."""

from __future__ import annotations

import re

from .titles import squash_subject


_DYNAMIC = re.compile(r"[$`]|__codexy_(?:command|process)_substitution__")
_PR_URL = re.compile(r"(?:^|/)pull/([1-9][0-9]*)(?:[/?#]|$)")
_VALUE_OPTIONS = frozenset(
    {
        "--body",
        "--body-file",
        "--match-head-commit",
        "--repo",
        "--subject",
        "-b",
    }
)


def forbidden(args: list[str]) -> bool:
    if "--squash" not in args:
        return False
    present, subject = _option(args, "--subject")
    if not present:
        return True
    number = _selector(args)
    return (
        not isinstance(subject, str)
        or number is None
        or not squash_subject(subject, number)
    )


def _option(args: list[str], name: str) -> tuple[bool, str | None]:
    found: str | None = None
    present = False
    index = 0
    while index < len(args):
        token = args[index]
        if token == name:
            if present or index + 1 >= len(args):
                return True, None
            present, found = True, args[index + 1]
            index += 2
            continue
        if token.startswith(name + "="):
            if present:
                return True, None
            present, found = True, token.split("=", 1)[1]
        index += 1
    return present, None if found is None or _DYNAMIC.search(found) else found


def _selector(args: list[str]) -> int | None:
    end_of_options = False
    index = 0
    while index < len(args):
        token = args[index]
        if not end_of_options and token == "--":
            end_of_options = True
            index += 1
            continue
        if not end_of_options and token in _VALUE_OPTIONS:
            index += 2
            continue
        if not end_of_options and any(
            token.startswith(option + "=") for option in _VALUE_OPTIONS
        ):
            index += 1
            continue
        if not end_of_options and token.startswith("-"):
            index += 1
            continue
        if token.isdigit() and int(token) > 0:
            return int(token)
        match = _PR_URL.search(token)
        if match is not None:
            return int(match.group(1))
        index += 1
    return None
