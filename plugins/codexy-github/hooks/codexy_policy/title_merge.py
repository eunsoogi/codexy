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
_BOOLEAN_TRUE = frozenset({"1", "t", "T", "true", "True", "TRUE"})
_BOOLEAN_FALSE = frozenset({"0", "f", "F", "false", "False", "FALSE"})
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
        "-F",
        "-R",
        "-t",
    }
)


def forbidden(args: list[str]) -> bool:
    if args in (["--help"], ["-h"]):
        return False
    auto_merge, disable_auto_merge, methods, unresolved = _merge_options(args)
    if auto_merge or len(methods) > 1 or unresolved:
        return True
    # Disabling queued auto-merge does not create a merge commit or squash subject.
    if disable_auto_merge and not methods:
        return False
    if methods and methods[0] in {"merge", "rebase"}:
        return False
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


def _merge_options(args: list[str]) -> tuple[bool, bool, list[str], bool]:
    auto_values: list[bool] = []
    disable_auto_values: list[bool] = []
    method_values: dict[str, bool] = {}
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
            auto_values.append(True)
        elif not end_of_options and token.startswith("--auto="):
            value = _boolean_value(token.partition("=")[2])
            if value is None:
                unresolved = True
            else:
                if auto_values and auto_values[-1] != value:
                    unresolved = True
                auto_values.append(value)
        elif not end_of_options and token == "--disable-auto":
            disable_auto_values.append(True)
        elif not end_of_options and token.startswith("--disable-auto="):
            value = _boolean_value(token.partition("=")[2])
            if value is None:
                unresolved = True
            else:
                if disable_auto_values and disable_auto_values[-1] != value:
                    unresolved = True
                disable_auto_values.append(value)
        elif not end_of_options:
            option, separator, raw_value = token.partition("=")
            method = _METHOD_OPTIONS.get(option if separator else token)
            if method is not None:
                value = True if not separator else _boolean_value(raw_value)
                if value is None:
                    unresolved = True
                else:
                    previous = method_values.get(method)
                    if previous is not None and previous != value:
                        unresolved = True
                    method_values[method] = value
        # Unknown dynamic tokens could stand for a merge-method flag.
        unresolved = unresolved or bool(_DYNAMIC.search(token))
        index += 1
    auto_merge = auto_values[-1] if auto_values else False
    disable_auto_merge = disable_auto_values[-1] if disable_auto_values else False
    methods = [method for method, enabled in method_values.items() if enabled]
    return auto_merge, disable_auto_merge, methods, unresolved


def _boolean_value(value: str) -> bool | None:
    # Recognize literal CLI boolean values; unknown or dynamic values stay unresolved.
    if value in _BOOLEAN_TRUE:
        return True
    if value in _BOOLEAN_FALSE:
        return False
    return None


def _option(args: list[str], names: tuple[str, ...]) -> tuple[bool, str | None]:
    found: str | None = None
    present = False
    value_options = _VALUE_OPTIONS.difference(names)
    end_of_options = False
    index = 0
    while index < len(args):
        token = args[index]
        if not end_of_options and token == "--":
            end_of_options = True
            index += 1
            continue
        # Do not reinterpret a value such as `--subject=...` as an option.
        if not end_of_options and token in value_options:
            index += 2
            continue
        if not end_of_options and any(
            token.startswith(name + "=") for name in value_options
        ):
            index += 1
            continue
        if not end_of_options and token in names:
            if present or index + 1 >= len(args):
                return True, None
            present, found = True, args[index + 1]
            index += 2
            continue
        if not end_of_options and any(token.startswith(name + "=") for name in names):
            if present:
                return True, None
            present, found = True, token.split("=", 1)[1]
        elif (
            not end_of_options
            and "-t" in names
            and token.startswith("-t")
            and len(token) > 2
        ):
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
