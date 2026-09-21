"""Literal object fields shared by bounded nested title inspection."""

from __future__ import annotations

from .title_nested_parser import Token, matching, value_end


MISSING = object()
DYNAMIC = object()


def object_fields(tokens: list[Token]) -> dict[str, object] | None:
    if tokens[0].value != "{" or matching(tokens, 0, "{", "}") != len(tokens) - 1:
        return None
    result: dict[str, object] = {}
    index = 1
    while index < len(tokens) - 1:
        key = tokens[index]
        if (
            key.kind not in {"identifier", "string"}
            or index + 1 >= len(tokens)
            or tokens[index + 1].value != ":"
        ):
            return None
        end = value_end(tokens, index + 2, len(tokens) - 1)
        if key.value in result:
            return None
        result[key.value] = _value(tokens[index + 2 : end])
        index = end
        if index == len(tokens) - 1:
            break
        if tokens[index].value != ",":
            return None
        index += 1
    return result


def _value(tokens: list[Token]) -> object:
    if len(tokens) == 1 and tokens[0].kind == "string":
        return tokens[0].value
    if (
        len(tokens) == 1
        and tokens[0].kind == "identifier"
        and tokens[0].value == "null"
    ):
        return None
    if len(tokens) == 1 and tokens[0].kind == "number":
        try:
            return int(tokens[0].value)
        except ValueError:
            return DYNAMIC
    if len(tokens) == 1 and tokens[0].kind == "identifier":
        return DYNAMIC
    return DYNAMIC
