"""Conservative handling for opaque shell syntax and policy selectors."""

from __future__ import annotations

import re
import shlex

from .execution_context import SINGLE_QUOTED_DOLLAR, assignment

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


def github_opaque(command: str) -> bool:
    return re.search(
        r"(?:^|[;&|()\s])gh(?=$|[;&|()\s])|\bGH_REPO\s*=", command
    ) is not None


def destructive_opaque(command: str) -> bool:
    return re.search(
        r"(?:^|[;&|()\s])(?:git|cd|source|\.|rm|pushd|popd)(?=$|[;&|()\s])"
        r"|\b(?:GIT_DIR|GIT_COMMON_DIR)\s*=",
        command,
    ) is not None
