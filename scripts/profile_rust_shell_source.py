"""Extract executable shell command tokens without running workflow steps."""

from __future__ import annotations

import shlex

SHELL_SEPARATORS = frozenset("();&|{}")


def shell_commands(command: str) -> list[list[str]]:
    lexer = shlex.shlex(shell_source(command), posix=True, punctuation_chars="();&|{}")
    lexer.whitespace_split = True
    lexer.commenters = "#"
    commands: list[list[str]] = []
    current: list[str] = []
    try:
        for token in lexer:
            if token and set(token) <= SHELL_SEPARATORS:
                if current:
                    commands.append(current)
                    current = []
            else:
                current.append(token)
    except ValueError:
        return commands
    if current:
        commands.append(current)
    return commands


def shell_source(command: str) -> str:
    source = strip_heredoc_data(command).replace("\\\r\n", "").replace("\\\n", "")
    return separate_command_lines(mask_multiline_quotes(source))


def strip_heredoc_data(command: str) -> str:
    output: list[str] = []
    pending: list[tuple[str, bool]] = []
    quote: str | None = None
    for line in command.splitlines(keepends=True):
        body = line.rstrip("\r\n")
        ending = line[len(body) :]
        if pending:
            delimiter, strip_tabs = pending[0]
            candidate = body.lstrip("\t") if strip_tabs else body
            if candidate == delimiter:
                pending.pop(0)
            output.append(ending)
            continue
        delimiters, quote = heredoc_delimiters(body, quote)
        pending.extend(delimiters)
        output.append(line)
    return "".join(output)


def heredoc_delimiters(
    line: str, quote: str | None
) -> tuple[list[tuple[str, bool]], str | None]:
    delimiters: list[tuple[str, bool]] = []
    index = 0
    while index < len(line):
        character = line[index]
        if character == "\\":
            index += 2
        elif quote is not None:
            quote = None if character == quote else quote
            index += 1
        elif character in "'\"":
            quote = character
            index += 1
        elif line.startswith("<<", index):
            index += 2
            strip_tabs = index < len(line) and line[index] == "-"
            index += int(strip_tabs)
            while index < len(line) and line[index].isspace():
                index += 1
            delimiter, index = heredoc_delimiter(line, index)
            if delimiter:
                delimiters.append((delimiter, strip_tabs))
        else:
            index += 1
    return delimiters, quote


def heredoc_delimiter(line: str, index: int) -> tuple[str, int]:
    if index >= len(line):
        return "", index
    quote = line[index] if line[index] in "'\"" else None
    if quote is not None:
        end = line.find(quote, index + 1)
        return (line[index + 1 : end], end + 1) if end >= 0 else ("", len(line))
    end = index
    while end < len(line) and not line[end].isspace() and line[end] not in ";|&(){}<>":
        end += 1
    return line[index:end], end


def mask_multiline_quotes(source: str) -> str:
    characters = list(source)
    quote: str | None = None
    start = 0
    multiline = False
    index = 0
    while index < len(characters):
        character = characters[index]
        if character == "\\":
            index += 2
        elif quote is not None:
            multiline = multiline or character in "\r\n"
            if character == quote:
                if multiline:
                    for position in range(start, index + 1):
                        characters[position] = " "
                quote = None
            index += 1
        elif character in "'\"":
            quote = character
            start = index
            multiline = False
            index += 1
        else:
            index += 1
    if quote is not None and multiline:
        for position in range(start, len(characters)):
            characters[position] = " "
    return "".join(characters)


def separate_command_lines(source: str) -> str:
    characters: list[str] = []
    quote: str | None = None
    index = 0
    while index < len(source):
        character = source[index]
        if character == "\\":
            characters.extend(source[index : index + 2])
            index += 2
        elif quote is not None:
            quote = None if character == quote else quote
            characters.append(character)
            index += 1
        elif character in "'\"":
            quote = character
            characters.append(character)
            index += 1
        elif character in "\r\n":
            characters.append(";")
            index += 1
        else:
            characters.append(character)
            index += 1
    return "".join(characters)
