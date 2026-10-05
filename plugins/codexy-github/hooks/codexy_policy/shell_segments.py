"""Shared parsed command-position walk for admission policy concerns."""

from __future__ import annotations

import shlex
from dataclasses import dataclass

from .execution_context import assignment
from .shell_redirections import (
    OPERATORS,
    separate_lines,
    strip_redirections,
)
from .shell_reflog import protect as protect_reflog, restore as restore_reflog
from .shell_heredoc import without_python_script_heredoc_bodies

CONTROL_WORDS = frozenset(
    "if then elif else fi for in while until do done case esac".split()
)
# Private markers keep quoted redirection and reflog data distinct from active shell operators.


def tokenize(command: str, has_substitutions: bool = False) -> list[str] | None:
    """Keep parent substitution context after opaque syntax becomes a placeholder."""
    command = without_python_script_heredoc_bodies(command)
    try:
        lexer = shlex.shlex(
            protect_reflog(separate_lines(command)),
            posix=True,
            punctuation_chars=";&|(){}<>",
        )
        lexer.whitespace_split, lexer.commenters = True, ""
        return strip_redirections(
            restore_reflog(list(lexer)),
            has_substitutions or bool(opaque_syntax(command).substitutions),
        )
    except ValueError:
        return None


@dataclass(frozen=True)
class OpaqueSyntax:
    """Executable shell syntax after literal and comment payloads are removed."""

    command: str
    substitutions: tuple[str, ...]
    control: bool


def segments(command: str) -> tuple[tuple[str, ...], ...] | None:
    """Return command-position segments; quoted text remains argument data."""
    tokens = tokenize(command)
    if tokens is None:
        return None
    result: list[tuple[str, ...]] = []
    current: list[str] = []
    command_start = True
    for token in [*tokens, ";"]:
        if token in OPERATORS:
            if current:
                result.append(tuple(current))
            current, command_start = [], True
        elif command_start and token.casefold() in CONTROL_WORDS:
            continue
        else:
            current.append(token)
            if command_start and token != "!" and not assignment(token):
                command_start = False
    return tuple(result)


def command_tokens(tokens: tuple[str, ...]) -> tuple[str, ...]:
    """Remove shell prefixes before inspecting the parsed command position."""
    index = 0
    while index < len(tokens) and (tokens[index] == "!" or assignment(tokens[index])):
        index += 1
    return tokens[index:]


def opaque_syntax(command: str) -> OpaqueSyntax:
    """Expose only executable substitutions and controls, never quoted data."""
    command = without_python_script_heredoc_bodies(command)
    result, code, substitutions = [], [], []
    quote, escaped, index = None, False, 0
    while index < len(command):
        char = command[index]
        if escaped:
            result.append(char)
            code.append(" ")
            escaped = False
        elif char == "\\" and quote != "'":
            result.append(char)
            code.append(" ")
            escaped = True
        elif char in {"'", '"'}:
            quote = None if quote == char else char if quote is None else quote
            result.append(char)
            code.append(" ")
        elif quote is None and char == "#" and _comment_start(command, index):
            while index < len(command) and command[index] != "\n":
                result.append(command[index])
                code.append(" ")
                index += 1
            continue
        elif (
            quote != "'"
            and (char == "$" or quote is None and char in "<>")
            and command[index + 1 : index + 2] == "("
        ):
            end = _substitution_end(command, index + 2)
            if end is None:
                result.append(char)
                code.append(" ")
            else:
                substitutions.append(command[index + 2 : end])
                result.append(
                    "__codexy_process_substitution__"
                    if char in "<>"
                    else "__codexy_command_substitution__"
                )
                code.append(" ")
                index = end
        elif quote != "'" and char == "`":
            end = _backtick_end(command, index + 1)
            if end is None:
                result.append(char)
                code.append(" ")
            else:
                substitutions.append(command[index + 1 : end])
                result.append("__codexy_command_substitution__")
                code.append(" ")
                index = end
        else:
            result.append(char)
            code.append(char if quote is None else " ")
        index += 1
    words = " ".join("".join(code).replace(";", " ").split())
    control = any(f" {word} " in f" {words} " for word in CONTROL_WORDS)
    return OpaqueSyntax("".join(result), tuple(substitutions), control)


def _comment_start(command: str, index: int) -> bool:
    return index == 0 or command[index - 1].isspace() or command[index - 1] in ";&|(){}"


def _substitution_end(command: str, index: int) -> int | None:
    depth, quote, escaped = 1, None, False
    while index < len(command):
        char = command[index]
        if escaped:
            escaped = False
        elif char == "\\" and quote != "'":
            escaped = True
        elif char in {"'", '"'}:
            quote = None if quote == char else char if quote is None else quote
        elif quote is None and char in "$<>" and command[index + 1 : index + 2] == "(":
            depth += 1
            index += 1
        elif quote is None and char == ")":
            depth -= 1
            if depth == 0:
                return index
        index += 1
    return None


def _backtick_end(command: str, index: int) -> int | None:
    escaped = False
    while index < len(command):
        if command[index] == "`" and not escaped:
            return index
        escaped = command[index] == "\\" and not escaped
        index += 1
    return None
