"""Token-structure helpers for nested GitHub call discovery."""

from __future__ import annotations

from .repository_github_exec_parser import ParseError, Token


def strip_parentheses(tokens: list[Token]) -> list[Token]:
    while len(tokens) >= 2 and tokens[0].value == "(" and tokens[-1].value == ")":
        if matching(tokens, 0, "(", ")") != len(tokens) - 1:
            break
        tokens = tokens[1:-1]
    return tokens


def is_tools(tokens: list[Token]) -> bool:
    tokens = strip_parentheses(tokens)
    return (
        len(tokens) == 1
        and tokens[0].kind == "identifier"
        and tokens[0].value == "tools"
    )


def call_open(tokens: list[Token], index: int) -> int | None:
    cursor = index + 1
    while cursor < len(tokens) and tokens[cursor].value == ")":
        cursor += 1
    if (
        cursor + 1 < len(tokens)
        and tokens[cursor].value == "?"
        and tokens[cursor + 1].value == "."
    ):
        cursor += 2
    if cursor >= len(tokens) or tokens[cursor].value != "(":
        return None
    return cursor


def global_identifier(tokens: list[Token], index: int) -> bool:
    return index == 0 or tokens[index - 1].value not in {".", "?"}


def _destructuring_alias(
    property_tokens: list[Token], prefix: str
) -> tuple[str, str | None] | None:
    if property_tokens and property_tokens[0].value == "[":
        close = matching(property_tokens, 0, "[", "]")
        if (
            close + 2 >= len(property_tokens)
            or property_tokens[close + 1].value != ":"
            or property_tokens[close + 2].kind != "identifier"
        ):
            return None
        alias = property_tokens[close + 2].value
        key = strip_parentheses(property_tokens[1:close])
        if (
            len(key) == 1
            and key[0].kind == "string"
            and key[0].value.startswith(prefix)
        ):
            return alias, key[0].value
        return alias, None
    if len(property_tokens) < 3:
        return None
    key, colon, alias = property_tokens[:3]
    if colon.value != ":" or alias.kind != "identifier":
        return None
    if key.kind in {"identifier", "string"} and key.value.startswith(prefix):
        return alias.value, key.value
    return None


def tools_destructuring_aliases(
    tokens: list[Token], prefix: str
) -> dict[str, str | None]:
    aliases: dict[str, str | None] = {}
    for open_index, token in enumerate(tokens):
        if token.value != "{":
            continue
        close = matching(tokens, open_index, "{", "}")
        if close + 2 >= len(tokens):
            continue
        if tokens[close + 1].value != "=" or not is_tools(
            tokens[close + 2 : close + 3]
        ):
            continue
        for property_tokens in arguments(tokens, open_index + 1, close):
            alias_info = _destructuring_alias(property_tokens, prefix)
            if alias_info is None:
                continue
            alias, tool = alias_info
            if alias not in aliases:
                aliases[alias] = tool
            elif aliases[alias] != tool:
                aliases[alias] = None
    return aliases


def arguments(tokens: list[Token], start: int, end: int) -> list[list[Token]]:
    result: list[list[Token]] = []
    begin = start
    depths = {"(": 0, "[": 0, "{": 0}
    pairs = {")": "(", "]": "[", "}": "{"}
    for index in range(start, end):
        value = tokens[index].value
        if value in depths:
            depths[value] += 1
        elif value in pairs:
            depths[pairs[value]] -= 1
        elif value == "," and not any(depths.values()):
            result.append(tokens[begin:index])
            begin = index + 1
    if begin != end:
        result.append(tokens[begin:end])
    return result


def matching(tokens: list[Token], start: int, opening: str, closing: str) -> int:
    depth = 0
    for index in range(start, len(tokens)):
        value = tokens[index].value
        if value == opening:
            depth += 1
        elif value == closing:
            depth -= 1
            if depth == 0:
                return index
    raise ParseError("unbalanced expression")
