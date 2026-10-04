"""Quoted here-document handling for the shared shell command parser."""

from __future__ import annotations

import re
import shlex


_SHELL_OPERATORS = frozenset({";", "&&", "||", "|", "&", "(", ")", "{", "}"})
_CONTROL_WORDS = frozenset("if then elif else fi for while until do done".split())
_PYTHON_EXECUTABLE = re.compile(r"(?:python(?:\d+(?:\.\d+)?)?|pypy\d*)\Z")


def without_python_script_heredoc_bodies(command: str) -> str:
    """Keep Python stdin scripts out of the shell command grammar."""
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
        # Strip bodies only when every queued delimiter is quoted and the receiver is a direct Python reader.
        if (
            quote is not None
            or arithmetic_depth
            or _shell_line_continues(line, quote)
            or not line.endswith(("\n", "\r"))
        ):
            return command
        if any(not python_script for _, _, python_script in heredocs):
            return command

        body_end = index
        for delimiter, strip_tabs, _ in heredocs:
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
) -> tuple[list[tuple[str, bool, bool]] | None, str | None, int]:
    heredocs: list[tuple[str, bool, bool]] = []
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
            heredocs.append((*heredoc, _python_script_receiver(line[:header_start])))
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
    try:
        lexer = shlex.shlex(prefix, posix=True, punctuation_chars=";&|(){}<>")
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
    except ValueError:
        return False

    # Keep only the command after the last shell operator; an earlier Python command cannot own this body.
    current: list[str] = []
    for token in tokens:
        if token in _SHELL_OPERATORS:
            current = []
        else:
            current.append(token)
    while current and (
        current[0].casefold() in _CONTROL_WORDS
        or re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", current[0])
    ):
        del current[0]
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


def _shell_line_continues(line: str, quote: str | None) -> bool:
    if quote == "'":
        return False
    content = line.rstrip("\r\n")
    trailing_backslashes = len(content) - len(content.rstrip("\\"))
    return trailing_backslashes % 2 == 1


def _comment_start(command: str, index: int) -> bool:
    return index == 0 or command[index - 1].isspace() or command[index - 1] in ";&|(){}"
