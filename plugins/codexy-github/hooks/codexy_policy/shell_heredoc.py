"""Quoted here-document handling for the shared shell command parser."""

from __future__ import annotations


def without_quoted_heredoc_bodies(command: str) -> str:
    """Keep quoted here-document bodies out of the shell command grammar."""
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
        if (
            quote is not None
            or arithmetic_depth
            or _shell_line_continues(line, quote)
            or not line.endswith(("\n", "\r"))
        ):
            return command

        body_end = index
        for delimiter, strip_tabs in heredocs:
            while body_end < len(lines):
                terminator = lines[body_end].rstrip("\r\n")
                if strip_tabs:
                    terminator = terminator.lstrip("\t")
                if terminator == delimiter:
                    break
                body_end += 1
            if body_end == len(lines):
                return command
            body_end += 1

        changed = True
        index = body_end
        quote, arithmetic_depth = None, 0
    return "".join(output) if changed else command


def _heredoc_headers(
    line: str, quote: str | None, arithmetic_depth: int
) -> tuple[list[tuple[str, bool]] | None, str | None, int]:
    heredocs: list[tuple[str, bool]] = []
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
            parsed = _quoted_heredoc_delimiter(line, index)
            if parsed is None:
                return None, quote, arithmetic_depth
            heredoc, index = parsed
            heredocs.append(heredoc)
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


def _shell_line_continues(line: str, quote: str | None) -> bool:
    if quote == "'":
        return False
    content = line.rstrip("\r\n")
    trailing_backslashes = len(content) - len(content.rstrip("\\"))
    return trailing_backslashes % 2 == 1


def _comment_start(command: str, index: int) -> bool:
    return index == 0 or command[index - 1].isspace() or command[index - 1] in ";&|(){}"
