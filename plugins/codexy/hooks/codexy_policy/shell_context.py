"""Shared command-shape helpers for the admission shell policy."""

from __future__ import annotations

import os
from pathlib import Path


def changed_directory(tokens: list[str], cwd: str) -> str:
    while tokens and "=" in tokens[0] and not tokens[0].startswith("-"):
        tokens = tokens[1:]
    if not tokens or name(tokens[0]) != "cd":
        return cwd
    args = tokens[1:]
    while args and args[0] in {"-L", "-P"}:
        args = args[1:]
    if args[:1] == ["--"]:
        args = args[1:]
    return resolve_cwd(cwd, args[0]) if len(args) == 1 else cwd


def resolve_cwd(cwd: str, target: str) -> str:
    return os.path.abspath(os.path.join(cwd, target))


def command_option(value: str) -> bool:
    return value.lower() in {"-command", "/c"} or (value.startswith("-") and not value.startswith("--") and "c" in value.lower()[1:])


def flag(args: list[str], short: str, long: str) -> bool:
    return any(arg == long or arg.startswith(long + "=") or (arg.startswith("-") and not arg.startswith("--") and short in arg[1:]) for arg in args)


def name(value: str) -> str:
    command = Path(value).name.lower()
    return command[:-4] if command.endswith(".exe") else command
