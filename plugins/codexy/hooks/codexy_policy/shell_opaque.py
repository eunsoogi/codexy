"""Concern-neutral handling for opaque shell syntax."""

from __future__ import annotations

import re
import shlex

from .execution_context import ExecutionContext, SINGLE_QUOTED_DOLLAR, assignment
from .invocation import resolve as resolve_invocation

DYNAMIC_NAME = re.compile(r"\$(?:\{[A-Za-z_][A-Za-z0-9_]*\}|[A-Za-z_][A-Za-z0-9_]*)")
CONTROL_COMMAND_START = {"if", "then", "elif", "else", "while", "until", "do"}


def separate_lines(command: str) -> str:
    """Normalize supported line continuations and comments before tokenization."""
    result, quote, escaped, index = [], None, False, 0
    while index < len(command):
        char = command[index]
        if char == "\\" and quote != "'" and command[index + 1 : index + 2] == "\n":
            index += 2
            continue
        if escaped:
            result.append(char)
            escaped = False
        elif char == "\\" and quote != "'":
            result.append(char)
            escaped = True
        elif char in {"'", '"'}:
            quote = None if quote == char else char if quote is None else quote
            result.append(char)
        elif quote == "'" and char == "$":
            result.append(SINGLE_QUOTED_DOLLAR)
        elif quote is None and char == "#" and (not result or result[-1].isspace() or result[-1] in ";&|(){}"):
            while index < len(command) and command[index] != "\n":
                index += 1
            continue
        else:
            result.append(";" if char == "\n" and quote is None else char)
        index += 1
    return "".join(result)


def dynamic_control_executable(command: str) -> bool:
    lexer = shlex.shlex(separate_lines(command), posix=True, punctuation_chars=";&|(){}")
    lexer.whitespace_split, lexer.commenters = True, ""
    command_start = True
    for token in lexer:
        if token in {";", "&&", "||", "|", "&", "(", ")", "{", "}"} or token.casefold() in CONTROL_COMMAND_START:
            command_start = True
        elif command_start and (token == "!" or assignment(token)):
            continue
        else:
            if command_start and DYNAMIC_NAME.fullmatch(token):
                return True
            command_start = False
    return False


def contains_policy_executable(
    command: str, context: ExecutionContext, expected: str,
) -> bool:
    """Recognize a policy executable at an opaque command boundary."""
    try:
        lexer = shlex.shlex(separate_lines(command), posix=True, punctuation_chars=";&|(){}")
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
        command_start = True
        segment_start = 0
        index = 0
        while index < len(tokens):
            token = tokens[index]
            if token in {";", "&&", "||", "|", "&", "(", ")", "{", "}"} or token.casefold() in CONTROL_COMMAND_START:
                command_start = True
                segment_start = index + 1
            elif command_start and (token == "!" or assignment(token)):
                pass
            elif command_start:
                end = index + 1
                while end < len(tokens) and tokens[end] not in {";", "&&", "||", "|", "&", "(", ")", "{", "}"}:
                    end += 1
                invocation = resolve_invocation(tokens[segment_start:end], context)
                if invocation is not None and invocation.executable == expected:
                    return True
                if invocation is not None and (invocation.opaque or invocation.context.opaque_environment):
                    prefix_free = _without_prefix(tokens[segment_start:end])
                    fallback = resolve_invocation(prefix_free, context)
                    if fallback is None or fallback.opaque or fallback.executable == expected:
                        return True
                    if not fallback.available:
                        return True
                command_start = False
                index = end - 1
            index += 1
        return False
    except ValueError:
        return True


def _without_prefix(tokens: list[str]) -> list[str]:
    while tokens and (tokens[0] == "!" or assignment(tokens[0])):
        tokens = tokens[1:]
    return tokens
