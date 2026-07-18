from __future__ import annotations

import shlex
from pathlib import Path

from .repository_policy import repository_owned

OPERATORS = {";", "&&", "||", "|", "&"}
SHELLS = {"sh", "bash", "zsh", "dash", "pwsh", "powershell"}


def _tokens(command: str) -> list[str]:
    lexer = shlex.shlex(command, posix=True, punctuation_chars=";&|")
    lexer.whitespace_split = True
    lexer.commenters = ""
    return list(lexer)


def _segments(tokens: list[str]) -> list[list[str]]:
    result: list[list[str]] = []
    current: list[str] = []
    for token in tokens:
        if token in OPERATORS:
            if current:
                result.append(current)
                current = []
        else:
            current.append(token)
    if current:
        result.append(current)
    return result


def _unwrap(tokens: list[str]) -> list[str]:
    remaining = list(tokens)
    while remaining:
        command = Path(remaining[0]).name.lower()
        if command == "env":
            remaining.pop(0)
            while remaining and ("=" in remaining[0] or remaining[0].startswith("-")):
                remaining.pop(0)
        elif command == "command":
            remaining.pop(0)
        elif command == "sudo":
            remaining.pop(0)
            while remaining and remaining[0].startswith("-"):
                remaining.pop(0)
        else:
            break
    return remaining


def _has_flag(tokens: list[str], long: str, short: str = "") -> bool:
    return any(
        token == long
        or token.startswith(f"{long}=")
        or (short and token.startswith("-") and not token.startswith("--") and short in token[1:])
        for token in tokens
    )


def _segment_forbidden(tokens: list[str], depth: int) -> bool:
    tokens = _unwrap(tokens)
    if not tokens:
        return False
    command = Path(tokens[0]).name.lower()
    if command in SHELLS and depth < 3:
        lowered = [token.lower() for token in tokens]
        for selector in ("-c", "-command"):
            if selector in lowered:
                index = lowered.index(selector)
                return index + 1 >= len(tokens) or shell_forbidden(tokens[index + 1], depth + 1)
    if command == "git" and len(tokens) >= 2:
        operation, arguments = tokens[1], tokens[2:]
        if operation == "push":
            return _has_flag(arguments, "--force", "f") or _has_flag(
                arguments, "--force-with-lease"
            ) or any(argument.startswith("+") and ":" in argument for argument in arguments)
        if operation == "reset":
            return "--hard" in arguments
        if operation == "clean":
            return _has_flag(arguments, "--force", "f")
    if command == "gh" and tokens[1:3] == ["pr", "merge"]:
        return "--admin" in tokens[3:]
    if command == "rm":
        recursive = _has_flag(tokens[1:], "--recursive", "r")
        force = _has_flag(tokens[1:], "--force", "f")
        targets = [token for token in tokens[1:] if not token.startswith("-")]
        return recursive and force and any(target in {"/", "~", "$HOME"} for target in targets)
    return False


def shell_forbidden(command: str, depth: int = 0) -> bool:
    try:
        return any(_segment_forbidden(segment, depth) for segment in _segments(_tokens(command)))
    except ValueError:
        return True
