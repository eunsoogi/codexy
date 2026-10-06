"""Quoted here-document handling for the shared shell command parser."""

from __future__ import annotations

import re
from typing import Literal

from .shell_redirections import command_prefix

_PYTHON_EXECUTABLE = re.compile(r"(?:python(?:\d+(?:\.\d+)?)?|pypy\d*)\Z")


def without_python_script_heredoc_bodies(command: str) -> str:
    """Keep Python stdin scripts out of the shell command grammar."""
    return _without_heredoc_bodies(command, receiver="python")


def without_github_body_heredoc_bodies(command: str) -> str:
    """Keep only literal PR-body input out of the title command grammar."""
    return _without_heredoc_bodies(command, receiver="github-body")


def _without_heredoc_bodies(
    command: str, *, receiver: Literal["python", "github-body"]
) -> str:
    lines = command.splitlines(keepends=True)
    output: list[str] = []
    quote: str | None = None
    arithmetic_depth = 0
    index = 0
    changed = False
    while index < len(lines):
        line = lines[index]
        heredocs, quote, arithmetic_depth = _heredoc_headers(
            line, quote, arithmetic_depth
        )
        if heredocs is None:
            return command
        output.append(line)
        index += 1
        if not heredocs:
            continue
        # Leave scripts and unknown receivers in the grammar; strip only proven data input.
        if (
            quote is not None
            or arithmetic_depth
            or _shell_line_continues(line, quote)
            or not line.endswith(("\n", "\r"))
        ):
            return command
        if receiver == "python" and any(
            not is_python for _, _, is_python, _ in heredocs
        ):
            return command
        if receiver == "github-body" and any(
            not is_github_body for _, _, _, is_github_body in heredocs
        ):
            return command

        body_end = index
        for delimiter, strip_tabs, _, _ in heredocs:
            while body_end < len(lines):
                terminator = lines[body_end].rstrip("\r\n")
                if strip_tabs:
                    terminator = terminator.lstrip("\t")
                if terminator == delimiter:
                    break
                body_end += 1
            if body_end == len(lines):
                return command
            changed = True
            body_end += 1

        index = body_end
        quote, arithmetic_depth = None, 0
    return "".join(output) if changed else command


def _heredoc_headers(
    line: str, quote: str | None, arithmetic_depth: int
) -> tuple[list[tuple[str, bool, bool, bool]] | None, str | None, int]:
    heredocs: list[tuple[str, bool, bool, bool]] = []
    index = 0
    while index < len(line):
        char = line[index]
        if char == "\\" and quote != "'":
            index = min(index + 2, len(line))
            continue
        if char in {"'", '"'}:
            if quote is None:
                quote = char
            elif quote == char:
                quote = None
            index += 1
            continue
        if quote is not None:
            index += 1
            continue
        if char == "#" and _comment_start(line, index):
            break
        if arithmetic_depth:
            if line[index : index + 2] == "((":
                arithmetic_depth += 2
                index += 2
            elif line[index : index + 2] == "))":
                arithmetic_depth = max(0, arithmetic_depth - 2)
                index += 2
            else:
                if char == "(":
                    arithmetic_depth += 1
                elif char == ")":
                    arithmetic_depth = max(0, arithmetic_depth - 1)
                index += 1
            continue
        if line[index : index + 2] == "((":
            arithmetic_depth = 2
            index += 2
            continue
        if line[index : index + 2] == "<<":
            if line[index + 2 : index + 3] == "<":
                index += 3
                continue
            header_start = index
            parsed = _quoted_heredoc_delimiter(line, index)
            if parsed is None:
                return None, quote, arithmetic_depth
            heredoc, index = parsed
            prefix = line[:header_start]
            heredocs.append(
                (
                    *heredoc,
                    _python_script_receiver(prefix),
                    _github_body_stdin_receiver(prefix),
                )
            )
            continue
        index += 1
    return heredocs, quote, arithmetic_depth


def _quoted_heredoc_delimiter(
    line: str, index: int
) -> tuple[tuple[str, bool], int] | None:
    index += 2
    strip_tabs = line[index : index + 1] == "-"
    if strip_tabs:
        index += 1
    while line[index : index + 1] in {" ", "\t"}:
        index += 1

    quote = line[index : index + 1]
    if quote not in {"'", '"'}:
        return None
    index += 1
    start = index
    while index < len(line):
        char = line[index]
        if char == quote:
            break
        if char in "\r\n" or (quote == '"' and char in {"$", "`", "\\"}):
            return None
        index += 1

    if index == len(line) or index == start:
        return None
    delimiter = line[start:index]
    index += 1
    if index < len(line) and not (line[index].isspace() or line[index] in ";|&(){}<>"):
        return None
    return (delimiter, strip_tabs), index


def _python_script_receiver(prefix: str) -> bool:
    """Recognize a direct Python command that reads its script from stdin."""
    current = command_prefix(prefix)
    if not current:
        return False
    executable = current[0].rsplit("/", 1)[-1]
    if _PYTHON_EXECUTABLE.fullmatch(executable) is None:
        return False
    arguments = current[1:]
    if any(option in {"-c", "-m"} for option in arguments):
        return False
    return "-" in arguments or not any(
        not argument.startswith("-") for argument in arguments
    )


def _github_body_stdin_receiver(prefix: str) -> bool:
    """Recognize PR edits that explicitly consume standard input as body text."""
    current = command_prefix(prefix)
    if (
        current is None
        or len(current) < 3
        or current[0].rsplit("/", 1)[-1].casefold() != "gh"
        or [part.casefold() for part in current[1:3]] != ["pr", "edit"]
    ):
        return False

    reads_stdin = False
    arguments = current[3:]
    index = 0
    while index < len(arguments):
        token = arguments[index]
        if token == "--":
            break
        if token in {"--body-file", "-F"}:
            if index + 1 == len(arguments):
                return False
            reads_stdin = arguments[index + 1] == "-"
            index += 2
        elif token.startswith("--body-file="):
            reads_stdin = token.partition("=")[2] == "-"
            index += 1
        elif token.startswith("-F") and token != "-F":
            reads_stdin = token[2:] == "-"
            index += 1
        else:
            index += 1
    return reads_stdin


def _shell_line_continues(line: str, quote: str | None) -> bool:
    if quote == "'":
        return False
    content = line.rstrip("\r\n")
    trailing_backslashes = len(content) - len(content.rstrip("\\"))
    return trailing_backslashes % 2 == 1


def _comment_start(command: str, index: int) -> bool:
    return index == 0 or command[index - 1].isspace() or command[index - 1] in ";&|(){}"
