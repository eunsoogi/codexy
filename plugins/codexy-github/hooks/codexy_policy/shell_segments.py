"""Shared parsed command-position walk for admission policy concerns."""

from __future__ import annotations

import shlex
from collections.abc import Iterator
from dataclasses import dataclass

from .execution_context import SINGLE_QUOTED_DOLLAR, assignment

QUOTED_REDIRECTIONS = {"<": "\ue001", ">": "\ue002"}
REDIRECTION_FD, UNSAFE_REDIRECTION = "\ue003", "\ue004"

CONTROL_WORDS = frozenset(
    "if then elif else fi for in while until do done case esac".split()
)
OPERATORS = frozenset({";", "&&", "||", "|", "&", "(", ")", "{", "}"})


def tokenize(command: str) -> list[str] | None:
    try:
        lexer = shlex.shlex(
            separate_lines(command), posix=True, punctuation_chars=";&|(){}<>"
        )
        lexer.whitespace_split, lexer.commenters = True, ""
        return _strip_redirections(list(lexer))
    except ValueError:
        return None


@dataclass(frozen=True)
class OpaqueSyntax:
    """Executable shell syntax after literal and comment payloads are removed."""

    command: str
    substitutions: tuple[str, ...]
    control: bool


def separate_lines(command: str) -> str:
    """Normalize supported continuations and comments before shell tokenization."""
    result, quote, escaped, index = [], None, False, 0
    while index < len(command):
        char = command[index]
        if char == "\\" and quote != "'" and command[index + 1 : index + 2] == "\n":
            index += 2
            continue
        if escaped:
            result.append(QUOTED_REDIRECTIONS[char] if char in "<>" else char)
            escaped = False
        elif char == "\\" and quote != "'":
            result.append(char)
            escaped = True
        elif char in {"'", '"'}:
            quote = None if quote == char else char if quote is None else quote
            result.append(char)
        elif quote is not None and char in "<>":
            result.append(QUOTED_REDIRECTIONS[char])
        elif quote == "'" and char == "$":
            result.append(SINGLE_QUOTED_DOLLAR)
        elif quote is None and char in "<>":
            _mark_redirection_fd(result)
            result.append(char)
        elif (
            quote is None
            and char == "#"
            and (not result or result[-1].isspace() or result[-1] in ";&|(){}")
        ):
            while index < len(command) and command[index] != "\n":
                index += 1
            continue
        elif char == "\n" and quote is None:
            while result and result[-1].isspace():
                result.pop()
            if result and result[-1] != ";":
                result.append(";")
        else:
            result.append(char)
        index += 1
    return "".join(result)


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


def _mark_redirection_fd(result: list[str]) -> None:
    start = len(result)
    while start and result[start - 1].isdigit():
        start -= 1
    if start < len(result) and (start == 0 or result[start - 1].isspace()):
        result.insert(start, REDIRECTION_FD)


def _is_redirection(token: str) -> bool:
    return any(char in "<>" for char in token) and set(token) <= set("<>&|-")


def _strip_redirections(tokens: list[str]) -> list[str] | None:
    result: list[str] = []
    iterator = iter(tokens)
    for token in iterator:
        if token.startswith(REDIRECTION_FD) and token[1:].isdigit():
            token = next(iterator, None)
            if token is None:
                return None
        if _is_redirection(token):
            target = next(iterator, None)
            if target is None or target in OPERATORS:
                return None
            if not (
                token.startswith("<")
                and ">" not in token
                or token in {">", ">>", ">|", "&>", "&>>"}
                and target == "/dev/null"
                or token in {">&", ">&-"}
                and (target.isdigit() or target in {"-", "/dev/null"})
            ):
                result.append(UNSAFE_REDIRECTION)
        else:
            result.append(token.replace("\ue001", "<").replace("\ue002", ">"))
    return result


def command_tokens(tokens: tuple[str, ...]) -> tuple[str, ...]:
    """Remove shell prefixes before inspecting the parsed command position."""
    index = 0
    while index < len(tokens) and (tokens[index] == "!" or assignment(tokens[index])):
        index += 1
    return tokens[index:]


def command_segments(command: str) -> Iterator[tuple[str, ...]]:
    """Iterate parsed segments, raising no hidden raw-token policy decisions."""
    yield from segments(command) or ()


def opaque_syntax(command: str) -> OpaqueSyntax:
    """Expose only executable substitutions and controls, never quoted data."""
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
        elif quote != "'" and char == "$" and command[index + 1 : index + 2] == "(":
            end = _substitution_end(command, index + 2)
            if end is None:
                result.append(char)
                code.append(" ")
            else:
                substitutions.append(command[index + 2 : end])
                result.append("__codexy_command_substitution__")
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
        elif quote is None and char == "$" and command[index + 1 : index + 2] == "(":
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
