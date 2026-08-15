"""Managed TOML-block parsing helpers for standalone-agent registration."""

from __future__ import annotations

BEGIN = "# BEGIN CODEXY MANAGED AGENTS"
END = "# END CODEXY MANAGED AGENTS"


def strip_managed_block(text: str) -> tuple[str, bool]:
    lines = text.splitlines(keepends=True)
    kept: list[str] = []
    multiline: str | None = None
    in_block = found = False
    for line in lines:
        marker = line.rstrip("\r\n")
        if multiline is None and marker == BEGIN:
            if in_block:
                return text, False
            in_block = found = True
            continue
        if multiline is None and marker == END:
            if not in_block:
                return text, False
            in_block = False
            continue
        if not in_block:
            kept.append(line)
        multiline, _ = multiline_state(line, multiline)
    if in_block:
        return text, False
    return "".join(kept), found


def multiline_state(line: str, state: str | None) -> tuple[str | None, int | None]:
    index, closed = 0, None
    while index < len(line):
        if state:
            if line.startswith(state, index) and (
                state == "'''" or not escaped(line, index)
            ):
                state = None
                index += 3
                closed = closed or index
            else:
                index += 1
            continue
        if line[index] == "#":
            break
        triple = next(
            (quote for quote in ('"""', "'''") if line.startswith(quote, index)), None
        )
        if triple:
            state, index = triple, index + 3
        elif line[index] in ('"', "'"):
            index = quoted_end(line, index) or len(line)
        else:
            index += 1
    return state, closed


def quoted_end(text: str, index: int) -> int | None:
    quote = text[index]
    index += 1
    while index < len(text):
        if quote == '"' and text[index] == "\\":
            index += 2
        elif text[index] == quote:
            return index + 1
        else:
            index += 1
    return None


def escaped(line: str, index: int) -> bool:
    slashes = 0
    while index > slashes and line[index - slashes - 1] == "\\":
        slashes += 1
    return slashes % 2 == 1
