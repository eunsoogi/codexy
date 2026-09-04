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
