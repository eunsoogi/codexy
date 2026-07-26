"""Fail-closed GraphQL operation classification for GitHub API admission."""

from __future__ import annotations

from .graphql_parser import document

STRING, NUMBER = "<string>", "<number>"


def mutation(query: str) -> bool | None:
    """Return whether a syntactically complete document defines a mutation."""
    tokens = _tokens(query)
    if not tokens:
        return None
    return document(tokens)


def _tokens(query: str) -> list[str] | None:
    tokens, index = [], 0
    while index < len(query):
        char = query[index]
        if char.isspace() or char == ",":
            index += 1
        elif char == "#":
            newline = query.find("\n", index)
            index = len(query) if newline < 0 else newline + 1
        elif query.startswith('"""', index):
            end = _block_string(query, index + 3)
            if end is None:
                return None
            tokens.append(STRING)
            index = end
        elif char == '"':
            end = _string(query, index + 1)
            if end is None:
                return None
            tokens.append(STRING)
            index = end
        elif char == "_" or char.isascii() and char.isalpha():
            end = index + 1
            while end < len(query) and (
                query[end] == "_" or query[end].isascii() and query[end].isalnum()
            ):
                end += 1
            tokens.append(query[index:end])
            index = end
        elif query.startswith("...", index):
            tokens.append("...")
            index += 3
        elif char in "!$&():=@[]{|}":
            tokens.append(char)
            index += 1
        elif char.isdigit() or char == "-":
            end = _number(query, index)
            if end is None:
                return None
            tokens.append(NUMBER)
            index = end
        else:
            return None
    return tokens


def _string(query: str, index: int) -> int | None:
    while index < len(query):
        char = query[index]
        if char == '"':
            return index + 1
        if char in "\r\n":
            return None
        if char == "\\":
            if index + 1 >= len(query):
                return None
            escape = query[index + 1]
            if escape in '"\\/bfnrt':
                index += 2
            elif escape == "u" and index + 5 < len(query) and all(
                digit in "0123456789abcdefABCDEF" for digit in query[index + 2 : index + 6]
            ):
                index += 6
            else:
                return None
        else:
            index += 1
    return None


def _block_string(query: str, index: int) -> int | None:
    while index < len(query):
        if query.startswith(r'\"""', index):
            index += 4
        elif query.startswith('"""', index):
            return index + 3
        else:
            index += 1
    return None


def _number(query: str, index: int) -> int | None:
    if query[index] == "-":
        index += 1
    start = index
    if index < len(query) and query[index] == "0":
        index += 1
    else:
        while index < len(query) and query[index].isdigit():
            index += 1
    if index == start:
        return None
    if index < len(query) and query[index] == ".":
        index += 1
        fraction = index
        while index < len(query) and query[index].isdigit():
            index += 1
        if index == fraction:
            return None
    if index < len(query) and query[index] in "eE":
        index += 1
        if index < len(query) and query[index] in "+-":
            index += 1
        exponent = index
        while index < len(query) and query[index].isdigit():
            index += 1
        if index == exponent:
            return None
    return index if index == len(query) or not (query[index].isalnum() or query[index] == "_") else None
