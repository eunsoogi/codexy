"""Small structural GraphQL document parser used by command admission."""

from __future__ import annotations


def document(tokens: list[str]) -> bool | None:
    """Classify a complete executable document, returning True for mutations."""
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token == "{":
            index = _selection(tokens, index)
        elif token == "fragment":
            index = _fragment(tokens, index)
        elif token in {"query", "mutation", "subscription"}:
            index = _definition(tokens, index)
            if index is None:
                return None
            if token == "mutation":
                return True
        else:
            return None
        if index is None:
            return None
    return False


def _definition(tokens: list[str], index: int) -> int | None:
    index += 1
    if index < len(tokens) and _name(tokens[index]):
        index += 1
    if index < len(tokens) and tokens[index] == "(":
        index = _variables(tokens, index)
    if index is None:
        return None
    index = _directives(tokens, index)
    return _selection(tokens, index) if index is not None else None


def _selection(tokens: list[str], index: int) -> int | None:
    end = _balanced(tokens, index)
    if end is None or index + 1 == end - 1:
        return None
    index += 1
    while index < end - 1:
        index = _spread(tokens, index) if tokens[index] == "..." else _field(tokens, index)
        if index is None or index >= end:
            return None
    return end if index == end - 1 else None


def _fragment(tokens: list[str], index: int) -> int | None:
    if index + 3 >= len(tokens) or not _name(tokens[index + 1]):
        return None
    if tokens[index + 2] != "on" or not _name(tokens[index + 3]):
        return None
    index = _directives(tokens, index + 4)
    return _selection(tokens, index) if index is not None else None


def _variables(tokens: list[str], index: int) -> int | None:
    end = _balanced(tokens, index)
    if end is None or end == index + 2:
        return None
    index += 1
    while index < end - 1:
        if index + 2 >= end or tokens[index] != "$" or not _name(tokens[index + 1]):
            return None
        if tokens[index + 2] != ":":
            return None
        index = _type(tokens, index + 3)
        if index is None:
            return None
        if index < end - 1 and tokens[index] == "=":
            index = _value(tokens, index + 1)
        index = _directives(tokens, index) if index is not None else None
        if index is None:
            return None
    return end if index == end - 1 else None


def _type(tokens: list[str], index: int) -> int | None:
    if index >= len(tokens):
        return None
    if tokens[index] == "[":
        end = _type(tokens, index + 1)
        if end is None or end >= len(tokens) or tokens[end] != "]":
            return None
        index = end + 1
    elif _name(tokens[index]):
        index += 1
    else:
        return None
    return index + 1 if index < len(tokens) and tokens[index] == "!" else index


def _directives(tokens: list[str], index: int) -> int | None:
    while index < len(tokens) and tokens[index] == "@":
        if index + 1 >= len(tokens) or not _name(tokens[index + 1]):
            return None
        index += 2
        if index < len(tokens) and tokens[index] == "(":
            index = _arguments(tokens, index)
            if index is None:
                return None
    return index


def _spread(tokens: list[str], index: int) -> int | None:
    index += 1
    if index >= len(tokens):
        return None
    if _name(tokens[index]) and tokens[index] != "on":
        return _directives(tokens, index + 1)
    if tokens[index] == "on":
        if index + 1 >= len(tokens) or not _name(tokens[index + 1]):
            return None
        index += 2
    index = _directives(tokens, index)
    return _selection(tokens, index) if index is not None else None


def _field(tokens: list[str], index: int) -> int | None:
    if index >= len(tokens) or not _name(tokens[index]):
        return None
    index += 1
    if index < len(tokens) and tokens[index] == ":":
        if index + 1 >= len(tokens) or not _name(tokens[index + 1]):
            return None
        index += 2
    if index < len(tokens) and tokens[index] == "(":
        index = _arguments(tokens, index)
    index = _directives(tokens, index) if index is not None else None
    if index is None:
        return None
    return _selection(tokens, index) if index < len(tokens) and tokens[index] == "{" else index


def _arguments(tokens: list[str], index: int) -> int | None:
    end = _balanced(tokens, index)
    if end is None or end == index + 2:
        return None
    index += 1
    while index < end - 1:
        if index + 1 >= end or not _name(tokens[index]) or tokens[index + 1] != ":":
            return None
        index = _value(tokens, index + 2)
        if index is None:
            return None
    return end if index == end - 1 else None


def _value(tokens: list[str], index: int) -> int | None:
    if index >= len(tokens):
        return None
    token = tokens[index]
    if token == "$":
        return index + 2 if index + 1 < len(tokens) and _name(tokens[index + 1]) else None
    if token in "[{":
        end = _balanced(tokens, index)
        if end is None:
            return None
        index += 1
        while index < end - 1:
            if token == "{":
                if index + 1 >= end or not _name(tokens[index]) or tokens[index + 1] != ":":
                    return None
                index += 2
            index = _value(tokens, index)
            if index is None:
                return None
        return end if index == end - 1 else None
    return index + 1 if token in {"<string>", "<number>"} or _name(token) else None


def _name(token: str) -> bool:
    return token not in {"<string>", "<number>", "..."} and token not in "{}()[]:$&!=@|"


def _balanced(tokens: list[str], index: int) -> int | None:
    if index >= len(tokens) or tokens[index] not in "{([":
        return None
    stack: list[str] = []
    while index < len(tokens):
        if not _nest(stack, tokens[index]):
            return None
        index += 1
        if not stack:
            return index
    return None


def _nest(stack: list[str], token: str) -> bool:
    pairs = {"}": "{", ")": "(", "]": "["}
    if token in {"{", "(", "["}:
        stack.append(token)
    elif token in pairs:
        if not stack or stack[-1] != pairs[token]:
            return False
        stack.pop()
    return True
