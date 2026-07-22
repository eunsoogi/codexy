"""Shared command-shape helpers for the admission shell policy."""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class DirectoryChange:
    cwd: str
    opaque: bool = False


def changed_directory(tokens: list[str], cwd: str) -> DirectoryChange:
    while tokens and "=" in tokens[0] and not tokens[0].startswith("-"):
        tokens = tokens[1:]
    if not tokens or name(tokens[0]) != "cd":
        return DirectoryChange(cwd)
    args = tokens[1:]
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


def command_option(value: str) -> bool:
    return value.lower() in {"-command", "/c"} or (value.startswith("-") and not value.startswith("--") and "c" in value.lower()[1:])


def flag(args: list[str], short: str, long: str) -> bool:
    return any(arg == long or arg.startswith(long + "=") or (arg.startswith("-") and not arg.startswith("--") and short in arg[1:]) for arg in args)


def name(value: str) -> str:
    command = Path(value).name.lower()
    return command[:-4] if command.endswith(".exe") else command
