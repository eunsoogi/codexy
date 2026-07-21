"""Extract executable shell command substitutions without running them."""

from __future__ import annotations


def command_substitutions(command: str) -> list[str]:
    substitutions: list[str] = []
    index = 0
    quote: str | None = None
    while index < len(command):
        character = command[index]
        if character == "\\":
            index += 2
        elif quote is not None and character == quote:
            quote = None
            index += 1
        elif quote == "'":
            index += 1
        elif quote is None and character in "'\"":
            quote = character
            index += 1
        elif quote is None and character == "#" and comment_start(command, index):
            newline = command.find("\n", index)
            index = len(command) if newline < 0 else newline + 1
        elif character == "`":
            end = command.find("`", index + 1)
            if end >= 0:
                substitutions.append(command[index + 1 : end])
                index = end + 1
            else:
                index += 1
        elif command.startswith("$(", index):
            end = closing_substitution(command, index + 2)
            if end >= 0:
                substitutions.append(command[index + 2 : end])
                index = end + 1
            else:
                index += 2
        else:
            index += 1
    return substitutions


def comment_start(command: str, index: int) -> bool:
    return index == 0 or command[index - 1].isspace() or command[index - 1] in ";|&(){}"


def closing_substitution(command: str, index: int) -> int:
    depth = 1
    quote: str | None = None
    while index < len(command):
        character = command[index]
        if character == "\\":
            index += 2
        elif quote is not None:
            quote = None if character == quote else quote
            index += 1
        elif character in "'\"":
            quote = character
            index += 1
        elif character == "(":
            depth += 1
            index += 1
        elif character == ")":
            depth -= 1
            if depth == 0:
                return index
            index += 1
        else:
            index += 1
    return -1
