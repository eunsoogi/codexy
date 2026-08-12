"""Shared command-shape helpers for the admission shell policy."""

from __future__ import annotations

import os
import re
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class DirectoryChange:
    cwd: str
    opaque: bool = False


REDIRECTION = re.compile(r"^\d*(?:>>|<>|>|<)(.*)$")


def changed_directory(tokens: list[str], cwd: str) -> DirectoryChange:
    while tokens and "=" in tokens[0] and not tokens[0].startswith("-"):
        tokens = tokens[1:]
    if not tokens:
        return DirectoryChange(cwd)
    command = name(tokens[0])
    if command in {"popd", "pushd"}:
        args = _without_redirections(tokens[1:])
        if command == "popd" or args is None or len(args) != 1 or not args[0] or args[0].startswith(("+", "-")):
            return DirectoryChange(cwd, True)
        return DirectoryChange(resolve_cwd(cwd, args[0]))
    if command != "cd":
        return DirectoryChange(cwd)
    args = _without_redirections(tokens[1:])
    if args is None:
        return DirectoryChange(cwd, True)
    mode, error_exit = "L", False
    while args and args[0].startswith("-") and args[0] != "-":
        option = args.pop(0)
        if option == "--":
            break
        if option.startswith("--") or any(flag not in "LPe@" for flag in option[1:]):
            return DirectoryChange(cwd)
        for flag in option[1:]:
            if flag in "LP":
                mode = flag
            elif flag == "e":
                error_exit = True
    if error_exit and mode != "P":
        return DirectoryChange(cwd)
    if not args or args == ["-"]:
        return DirectoryChange(cwd, True)
    if len(args) != 1 or not args[0]:
        return DirectoryChange(cwd)
    return DirectoryChange(resolve_cwd(cwd, args[0]))


def resolve_cwd(cwd: str, target: str) -> str:
    return os.path.abspath(os.path.join(cwd, target))


def _without_redirections(args: list[str]) -> list[str] | None:
    result, index = [], 0
    while index < len(args):
        match = REDIRECTION.fullmatch(args[index])
        if match is None:
            result.append(args[index])
            index += 1
        elif match.group(1):
            index += 1
        elif index + 1 < len(args):
            index += 2
        else:
            return None
    return result


def command_option(value: str) -> bool:
    return value.lower() in {"-command", "/c"} or (value.startswith("-") and not value.startswith("--") and "c" in value.lower()[1:])


def flag(args: list[str], short: str, long: str) -> bool:
    return any(arg == long or arg.startswith(long + "=") or (arg.startswith("-") and not arg.startswith("--") and short in arg[1:]) for arg in args)


def name(value: str) -> str:
    if value == ".":
        return value
    command = Path(value).name.lower()
    return command[:-4] if command.endswith(".exe") else command
