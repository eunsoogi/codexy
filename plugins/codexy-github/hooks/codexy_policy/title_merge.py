"""Literal GitHub CLI squash-subject inspection."""

from __future__ import annotations

import re

from .titles import squash_subject


_DYNAMIC = re.compile(r"[$`]|__codexy_(?:command|process)_substitution__")
_PR_URL = re.compile(r"(?:^|/)pull/([1-9][0-9]*)(?:[/?#]|$)")
_METHOD_OPTIONS = {
    "--merge": "merge",
    "-m": "merge",
    "--rebase": "rebase",
    "-r": "rebase",
    "--squash": "squash",
    "-s": "squash",
}
_VALUE_OPTIONS = frozenset(
    {
        "--author-email",
        "--body",
        "--body-file",
        "--match-head-commit",
        "--repo",
        "--subject",
        "-A",
        "-b",
        "-t",
    }
)


def forbidden(args: list[str]) -> bool:
    if args in (["--help"], ["-h"]):
        return False
    auto_merge, methods, unresolved = _merge_options(args)
    if auto_merge or len(methods) > 1:
        return True
    if methods and methods[0] in {"merge", "rebase"}:
        return False
    if unresolved:
        return True
    # Squash and default-method calls need a literal subject and unambiguous PR selector.
    present, subject = _option(args, ("--subject", "-t"))
    if not present:
        return True
    number = _selector(args)
    return (
        not isinstance(subject, str)
        or number is None
        or not squash_subject(subject, number)
    )


def _merge_options(args: list[str]) -> tuple[bool, list[str], bool]:
    auto_merge = False
    methods: list[str] = []
    unresolved = False
    end_of_options = False
    index = 0
    while index < len(args):
        token = args[index]
        if not end_of_options and token == "--":
            end_of_options = True
            index += 1
            continue
        if not end_of_options and token in _VALUE_OPTIONS:
            if index + 1 >= len(args):
                unresolved = True
            index += 2
            continue
        if not end_of_options and any(
            token.startswith(option + "=") for option in _VALUE_OPTIONS
        ):
            index += 1
            continue
        if not end_of_options and token == "--auto":
            auto_merge = True
        elif not end_of_options and token.startswith("--auto="):
            auto_merge = True
            unresolved = True
        elif not end_of_options:
            option, separator, _value = token.partition("=")
            method = _METHOD_OPTIONS.get(option if separator else token)
            if method is not None:
                methods.append(method)
                unresolved = unresolved or bool(separator)
        # Unknown dynamic tokens could stand for a merge-method flag.
        unresolved = unresolved or bool(_DYNAMIC.search(token))
        index += 1
    return auto_merge, methods, unresolved


def _option(args: list[str], names: tuple[str, ...]) -> tuple[bool, str | None]:
    found: str | None = None
    present = False
    index = 0
    while index < len(args):
        token = args[index]
        if token in names:
            if present or index + 1 >= len(args):
                return True, None
            present, found = True, args[index + 1]
            index += 2
            continue
        if any(token.startswith(name + "=") for name in names):
            if present:
                return True, None
            present, found = True, token.split("=", 1)[1]
        elif "-t" in names and token.startswith("-t") and len(token) > 2:
            if present:
                return True, None
            present, found = True, token[2:].removeprefix("=")
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
