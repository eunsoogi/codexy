"""Bounded GraphQL title inspection for GitHub mutation roots."""

from __future__ import annotations

from typing import NamedTuple

from .titles import issue_title, pr_title


class _Token(NamedTuple):
    kind: str
    value: str


_OPERATIONS = {
    "createIssue": (True, issue_title),
    "updateIssue": (False, issue_title),
    "createPullRequest": (True, pr_title),
    "updatePullRequest": (False, pr_title),
}
_DYNAMIC = object()


def forbidden(query: object) -> bool:
    if not isinstance(query, str):
        return True
    try:
        tokens = _tokenize(query)
        for index, token in enumerate(tokens):
            operation = _OPERATIONS.get(token.value) if token.kind == "name" else None
            if (
                operation is None
                or index + 1 >= len(tokens)
                or tokens[index + 1].value != "("
            ):
                continue
            close = _matching(tokens, index + 1)
            values = _title_values(tokens, index + 2, close)
            create, predicate = operation
            if create and (
                not values
                or any(
                    value is _DYNAMIC
                    or not isinstance(value, str)
                    or not predicate(value)
                    for value in values
                )
            ):
                return True
            if not create and any(
                value is _DYNAMIC
                or value is not None
                and (not isinstance(value, str) or not predicate(value))
                for value in values
            ):
                return True
        return False
    except ValueError:
        return True


def _title_values(tokens: list[_Token], start: int, end: int) -> list[object]:
    values: list[object] = []
    for index in range(start, end - 1):
        if (
            tokens[index].kind == "name"
            and tokens[index].value == "title"
            and tokens[index + 1].value == ":"
        ):
            values.append(_value(tokens[index + 2] if index + 2 < end else None))
    return values


def _value(token: _Token | None) -> object:
    if token is None:
        return _DYNAMIC
    if token.kind == "string":
        return token.value
    if token.kind == "name" and token.value == "null":
        return None
    return _DYNAMIC


def _matching(tokens: list[_Token], start: int) -> int:
    depth = 0
    for index in range(start, len(tokens)):
        if tokens[index].value == "(":
            depth += 1
        elif tokens[index].value == ")":
            depth -= 1
            if depth == 0:
                return index
    raise ValueError("unbalanced GraphQL arguments")


def _tokenize(source: str) -> list[_Token]:
    tokens: list[_Token] = []
    index = 0
    while index < len(source):
        char = source[index]
        if char.isspace() or char == ",":
            index += 1
        elif char == "#":
            newline = source.find("\n", index + 1)
            index = len(source) if newline < 0 else newline + 1
        elif source.startswith('"""', index):
            value, index = _block_string(source, index)
            tokens.append(_Token("string", value))
        elif char == '"':
            value, index = _string(source, index)
            tokens.append(_Token("string", value))
        elif char.isascii() and (char.isalpha() or char == "_"):
            end = index + 1
            while (
                end < len(source)
                and source[end].isascii()
                and (source[end].isalnum() or source[end] == "_")
            ):
                end += 1
            tokens.append(_Token("name", source[index:end]))
            index = end
        elif char.isdigit() or char == "-":
            end = index + 1
            while end < len(source) and source[end] in ".0123456789eE+-":
                end += 1
            tokens.append(_Token("number", source[index:end]))
            index = end
        else:
            tokens.append(_Token("punctuation", char))
            index += 1
    return tokens


def _string(source: str, index: int) -> tuple[str, int]:
    index += 1
    chars: list[str] = []
    escapes = {"b": "\b", "f": "\f", "n": "\n", "r": "\r", "t": "\t"}
    while index < len(source):
        char = source[index]
        if char == '"':
            return "".join(chars), index + 1
        if char in "\r\n":
            raise ValueError("newline in GraphQL string")
        if char == "\\":
            index += 1
            if index >= len(source):
                raise ValueError("unterminated GraphQL string")
            escaped = source[index]
            if escaped == "u":
                digits = source[index + 1 : index + 5]
                if len(digits) != 4:
                    raise ValueError("invalid GraphQL unicode escape")
                try:
                    chars.append(chr(int(digits, 16)))
                except ValueError as error:
                    raise ValueError("invalid GraphQL unicode escape") from error
                index += 5
                continue
            chars.append(escapes.get(escaped, escaped))
        else:
            chars.append(char)
        index += 1
    raise ValueError("unterminated GraphQL string")


def _block_string(source: str, index: int) -> tuple[str, int]:
    end = source.find('"""', index + 3)
    if end < 0:
        raise ValueError("unterminated GraphQL block string")
    return source[index + 3 : end], end + 3
