"""Token markers and safety checks for shell redirections."""

from __future__ import annotations

import re
import shlex

from .execution_context import SINGLE_QUOTED_DOLLAR, safe_output_redirection

QUOTED_REDIRECTIONS = {"<": "\ue001", ">": "\ue002"}
REDIRECTION_FD, UNSAFE_REDIRECTION = "\ue003", "\ue004"
PARSER_MARKERS = frozenset(
    {
        SINGLE_QUOTED_DOLLAR,
        *QUOTED_REDIRECTIONS.values(),
        REDIRECTION_FD,
        UNSAFE_REDIRECTION,
    }
)
OPERATORS = frozenset({";", "&&", "||", "|", "&", "(", ")", "{", "}"})
_CONTROL_WORDS = frozenset("if then elif else fi for while until do done".split())


def command_prefix(prefix: str) -> list[str] | None:
    """Return the active simple command before a here-document operator."""
    try:
        lexer = shlex.shlex(
            separate_lines(prefix), posix=True, punctuation_chars=";&|(){}<>"
        )
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
    except ValueError:
        return None

    without_redirections: list[str] = []
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token.startswith(REDIRECTION_FD) and token[1:].isdigit():
            index += 1
            if index == len(tokens):
                return None
            token = tokens[index]
        if _is_redirection(token):
            index += 2
            if index > len(tokens):
                return None
            continue
        without_redirections.append(token)
        index += 1

    current: list[str] = []
    for token in without_redirections:
        if token in OPERATORS:
            current = []
        else:
            current.append(token)
    while current and (
        current[0].casefold() in _CONTROL_WORDS
        or re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", current[0])
    ):
        del current[0]
    return current


def mark_redirection_fd(result: list[str]) -> None:
    start = len(result)
    while start and result[start - 1].isdigit():
        start -= 1
    if start < len(result) and (start == 0 or result[start - 1].isspace()):
        result.insert(start, REDIRECTION_FD)


def _is_redirection(token: str) -> bool:
    return any(char in "<>" for char in token) and set(token) <= set("<>&|-")


def separate_lines(command: str) -> str:
    """Normalize supported continuations and mark quoted redirection data."""
    result: list[str] = []
    quote: str | None = None
    escaped, index = False, 0
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
            mark_redirection_fd(result)
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
                _ = result.pop()
            if result and result[-1] != ";":
                result.append(";")
        else:
            result.append(char)
        index += 1
    return "".join(result)


def strip_redirections(
    tokens: list[str], has_substitutions: bool = False
) -> list[str] | None:
    result: list[str] = []
    iterator = iter(tokens)
    preceding_segment = False
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
                # Reject escaped parser markers before they can name a different filesystem path.
                and not any(marker in target for marker in PARSER_MARKERS)
                and safe_output_redirection(token, target)
                # Only private-temp exceptions depend on link-setup ordering; /dev/null stays safe.
                and (
                    target == "/dev/null"
                    or not preceding_segment
                    and not has_substitutions
                )
                or token in {">&", ">&-"}
                and (target.isdigit() or target in {"-", "/dev/null"})
            ):
                result.append(UNSAFE_REDIRECTION)
        else:
            result.append(token.replace("\ue001", "<").replace("\ue002", ">"))
            preceding_segment = preceding_segment or token in OPERATORS
    return result
