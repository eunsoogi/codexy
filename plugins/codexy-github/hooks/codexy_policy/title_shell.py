"""Title-only inspection for literal GitHub CLI and API shell commands."""

from __future__ import annotations

import re

from .shell_segments import segments
from .title_api import forbidden as api_forbidden
from .title_merge import forbidden as merge_forbidden
from .titles import issue_title, pr_title


_ASSIGNMENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
_DYNAMIC = re.compile(r"[$`]|__codexy_(?:command|process)_substitution__")


def forbidden(command: object, cwd: object = None) -> bool:
    if not isinstance(command, str):
        return False
    parsed = segments(command)
    if parsed is None:
        return bool(
            re.search(r"\bgh\s+(?:issue|pr)\s+(?:create|new|edit|merge)\b", command)
        )
    return any(_inspect_segment(segment, cwd) for segment in parsed)


def _inspect_segment(segment: tuple[str, ...], cwd: object) -> bool:
    tokens = _command_tokens(segment)
    if not tokens or tokens[0].rsplit("/", 1)[-1] != "gh":
        return False
    args = list(tokens[1:])
    if len(args) < 2:
        return False
    if args[0] == "api":
        return api_forbidden(args[1:], cwd)
    if args[:2] in (
        ["issue", "create"],
        ["issue", "new"],
        ["pr", "create"],
        ["pr", "new"],
    ):
        return _inspect_form(args[0], True, args[2:])
    if args[:2] in (["issue", "edit"], ["pr", "edit"]):
        return _inspect_form(args[0], False, args[2:])
    if args[:2] == ["pr", "merge"]:
        return merge_forbidden(args[2:])
    return False


def _inspect_form(kind: str, create: bool, args: list[str]) -> bool:
    present, value = _option(args, ("--title", "-t"))
    if not present:
        return create
    return not isinstance(value, str) or not (
        issue_title if kind == "issue" else pr_title
    )(value)


def _option(args: list[str], names: tuple[str, ...]) -> tuple[bool, str | None]:
    found: str | None = None
    present = False
    for index, token in enumerate(args):
        if token in names:
            if present or index + 1 >= len(args):
                return True, None
            present = True
            found = args[index + 1]
        elif any(token.startswith(name + "=") for name in names):
            if present:
                return True, None
            present, found = True, token.split("=", 1)[1]
        elif token.startswith("-t") and "-t" in names and len(token) > 2:
            if present:
                return True, None
            present, found = True, token[2:].removeprefix("=")
    return present, None if found is None or _DYNAMIC.search(found) else found


def _command_tokens(segment: tuple[str, ...]) -> tuple[str, ...]:
    index = 0
    while index < len(segment):
        token = segment[index]
        if token == "!" or _ASSIGNMENT.match(token):
            index += 1
            continue
        name = token.rsplit("/", 1)[-1]
        if name in {"command", "exec", "nohup", "nice", "time", "sudo", "timeout"}:
            index += 1
            if index < len(segment) and segment[index] == "--":
                index += 1
            continue
        if name == "env":
            index += 1
            while index < len(segment) and (
                _ASSIGNMENT.match(segment[index]) or segment[index].startswith("-")
            ):
                index += 1
            continue
        break
    return segment[index:]
