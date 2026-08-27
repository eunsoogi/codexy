"""Small structural GraphQL parser used by command admission."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Field:
    name: str
    arguments: tuple[tuple[str, object], ...] = ()
    selection: tuple["Field", ...] = ()
    alias: bool = False


@dataclass(frozen=True)
class Operation:
    kind: str
    selection: tuple[Field, ...]


def document(tokens: list[str]) -> bool | None:
    """Classify a complete document, retaining only the mutation bit."""
    parsed = parse_document(tokens)
    if parsed is None:
        return None
    return any(operation.kind == "mutation" for operation in parsed)


def parse_document(tokens: list[str]) -> tuple[Operation, ...] | None:
    operations: list[Operation] = []
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token == "fragment":
            index = _fragment(tokens, index)
        elif token == "{":
            selection, index = _selection(tokens, index)
            if selection is None:
                return None
            operations.append(Operation("query", selection))
        elif token in {"query", "mutation", "subscription"}:
            parsed = _operation(tokens, index)
            if parsed is None:
                return None
            operation, index = parsed
            operations.append(operation)
        else:
            return None
        if index is None:
            return None
    return tuple(operations) if operations else None


def _operation(tokens: list[str], index: int) -> tuple[Operation, int] | None:
    kind = tokens[index]
    index += 1
    if index < len(tokens) and _name(tokens[index]):
        index += 1
    if index < len(tokens) and tokens[index] == "(":
        index = _variables(tokens, index)
    if index is None:
        return None
    index = _directives(tokens, index)
    if index is None:
        return None
    selection, index = _selection(tokens, index)
    return None if selection is None else (Operation(kind, selection), index)


def _selection(tokens: list[str], index: int) -> tuple[tuple[Field, ...] | None, int]:
    if index >= len(tokens) or tokens[index] != "{":
        return None, index
    index += 1
    fields: list[Field] = []
    while index < len(tokens) and tokens[index] != "}":
        parsed = _spread(tokens, index) if tokens[index] == "..." else _field(tokens, index)
        if parsed is None:
            return None, index
        field, index = parsed
        fields.append(field)
    if not fields or index >= len(tokens):
        return None, index
    return tuple(fields), index + 1


def _field(tokens: list[str], index: int) -> tuple[Field, int] | None:
    if index >= len(tokens) or not _name(tokens[index]):
        return None
    name = tokens[index]
    aliased = False
    index += 1
    if index < len(tokens) and tokens[index] == ":":
        if index + 1 >= len(tokens) or not _name(tokens[index + 1]):
            return None
        name, index, aliased = tokens[index + 1], index + 2, True
    arguments: tuple[tuple[str, object], ...] = ()
    if index < len(tokens) and tokens[index] == "(":
        parsed = _arguments(tokens, index)
        if parsed is None:
            return None
        arguments, index = parsed
    index = _directives(tokens, index)
    if index is None:
        return None
    selection: tuple[Field, ...] = ()
    if index < len(tokens) and tokens[index] == "{":
        selection, index = _selection(tokens, index)
        if selection is None:
            return None
    return Field(name, arguments, selection, aliased), index


def _spread(tokens: list[str], index: int) -> tuple[Field, int] | None:
    index += 1
    if index >= len(tokens):
        return None
    if _name(tokens[index]) and tokens[index] != "on":
        index = _directives(tokens, index + 1)
        return None if index is None else (Field("..."), index)
    if tokens[index] == "on":
        if index + 1 >= len(tokens) or not _name(tokens[index + 1]):
            return None
        index = _directives(tokens, index + 2)
        if index is None:
            return None
        selection, index = _selection(tokens, index)
        return None if selection is None else (Field("...", selection=selection), index)
    return None


def _fragment(tokens: list[str], index: int) -> int | None:
    if index + 3 >= len(tokens) or not _name(tokens[index + 1]) or tokens[index + 2] != "on" or not _name(tokens[index + 3]):
        return None
    index = _directives(tokens, index + 4)
    if index is None:
        return None
    selection, index = _selection(tokens, index)
    return index if selection is not None else None


def _variables(tokens: list[str], index: int) -> int | None:
    if index >= len(tokens) or tokens[index] != "(":
        return None
    index += 1
    if index >= len(tokens) or tokens[index] == ")":
        return None
    while index < len(tokens) and tokens[index] != ")":
        if index + 2 >= len(tokens) or tokens[index] != "$" or not _name(tokens[index + 1]) or tokens[index + 2] != ":":
            return None
        index = _type(tokens, index + 3)
        if index is None:
            return None
        if index < len(tokens) and tokens[index] == "=":
            parsed = _parse_value(tokens, index + 1)
            index = None if parsed is None else parsed[1]
        index = _directives(tokens, index) if index is not None else None
        if index is None:
            return None
    return index + 1 if index < len(tokens) and tokens[index] == ")" else None


def _type(tokens: list[str], index: int) -> int | None:
    if index >= len(tokens):
        return None
    if tokens[index] == "[":
        index = _type(tokens, index + 1)
        if index is None or index >= len(tokens) or tokens[index] != "]":
            return None
        index += 1
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
            parsed = _arguments(tokens, index)
            if parsed is None:
                return None
            _, index = parsed
    return index


def _arguments(tokens: list[str], index: int) -> tuple[tuple[tuple[str, object], ...], int] | None:
    index += 1
    values: list[tuple[str, object]] = []
    while index < len(tokens) and tokens[index] != ")":
        if index + 1 >= len(tokens) or not _name(tokens[index]) or tokens[index + 1] != ":":
            return None
        name = tokens[index]
        if any(existing == name for existing, _ in values):
            return None
        parsed = _parse_value(tokens, index + 2)
        if parsed is None:
            return None
        value, index = parsed
        values.append((name, value))
    return (tuple(values), index + 1) if values and index < len(tokens) else None


def _parse_value(tokens: list[str], index: int) -> tuple[object, int] | None:
    if index >= len(tokens):
        return None
    token = tokens[index]
    if token == "$":
        if index + 1 >= len(tokens) or not _name(tokens[index + 1]):
            return None
        return ("variable", tokens[index + 1]), index + 2
    if token in "[{":
        close = "]" if token == "[" else "}"
        index += 1
        values: list[object] = []
        while index < len(tokens) and tokens[index] != close:
            if token == "{":
                if index + 1 >= len(tokens) or not _name(tokens[index]) or tokens[index + 1] != ":":
                    return None
                key = tokens[index]
                parsed = _parse_value(tokens, index + 2)
                if parsed is None:
                    return None
                value, index = parsed
                values.append((key, value))
            else:
                parsed = _parse_value(tokens, index)
                if parsed is None:
                    return None
                value, index = parsed
                values.append(value)
        if index >= len(tokens) or tokens[index] != close:
            return None
        return ("object" if token == "{" else "list", tuple(values)), index + 1
    return (token, index + 1) if token in {"<string>", "<number>"} or _name(token) else None


def _name(token: str) -> bool:
    return token not in {"<string>", "<number>", "..."} and token not in "{}()[]:$&!=@|"
