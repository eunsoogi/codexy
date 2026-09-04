"""Admission for GitHub connector calls nested inside ``functions.exec``."""

from __future__ import annotations

from typing import NamedTuple

from .connector import READ_OPERATIONS, connector_admitted
from .envelope import Request
from .repository_github_exec_literals import LiteralParser
from .repository_github_exec_parser import ParseError, Token, tokenize
from .repository_github_exec_structure import (
    arguments as _arguments,
    call_open as _call_open,
    global_identifier as _global_identifier,
    is_tools as _is_tools,
    matching as _matching,
    strip_parentheses as _strip_parentheses,
    tools_destructuring_aliases as _tools_destructuring_aliases,
)

MAX_CODE = 64 * 1024
MAX_EVAL_DEPTH = 8
TOOL_PREFIX = "mcp__codex_apps__github_"
UNINSPECTABLE = "UNINSPECTABLE_USE_DIRECT_ADMITTED_SURFACE"
POLICY = "POLICY_USE_DIRECT_ADMITTED_SURFACE"


class Call(NamedTuple):
    tokens: list[Token]
    tool: str
    argument_index: int


def forbidden(request: Request) -> bool | str:
    if not isinstance(request.tool_input, dict):
        return UNINSPECTABLE
    code = request.tool_input.get("code")
    if not isinstance(code, str) or len(code) > MAX_CODE:
        return UNINSPECTABLE
    try:
        calls = _calls(tokenize(code))
    except ParseError:
        return UNINSPECTABLE
    for call in calls:
        if call.tool.rsplit("github_", 1)[-1] in READ_OPERATIONS:
            continue
        try:
            parser = LiteralParser(call.tokens, call.argument_index)
            payload = parser.parse()
            parser.expect(")")
        except ParseError:
            return UNINSPECTABLE
        if not connector_admitted(call.tool, payload, request.cwd):
            return POLICY
    return False


def _calls(tokens: list[Token], depth: int = 0) -> list[Call]:
    if depth > MAX_EVAL_DEPTH:
        raise ParseError("evaluation nesting")
    calls: list[Call] = []
    indirect_depth = 0
    aliases = _tools_destructuring_aliases(tokens, TOOL_PREFIX)
    for index, token in enumerate(tokens):
        if token.kind == "dynamic":
            if token.value == "template_expression_start":
                indirect_depth += 1
            elif token.value == "template_expression_end":
                indirect_depth -= 1
            continue
        if token.kind != "identifier":
            continue
        if token.value == "tools":
            call = _tool_call(tokens, index, indirect=indirect_depth > 0)
            if call is not None:
                calls.append(call)
        elif token.value == "Reflect":
            call = _reflect_call(tokens, index)
            if call is not None:
                calls.append(call)
        elif token.value == "eval" and _global_identifier(tokens, index):
            calls.extend(_eval_calls(tokens, index, depth))
        elif token.value.startswith(TOOL_PREFIX):
            if not _direct_member(tokens, index) and not _property_key(tokens, index):
                if token.value.rsplit("github_", 1)[-1] not in READ_OPERATIONS:
                    raise ParseError("aliased mutation")
        elif token.value.startswith("github_") and not _property_key(tokens, index):
            if token.value[7:] not in READ_OPERATIONS:
                raise ParseError("aliased mutation name")
        elif (
            token.value in aliases
            and _global_identifier(tokens, index)
            and _call_open(tokens, index) is not None
        ):
            tool = aliases[token.value]
            if tool is None or tool.rsplit("github_", 1)[-1] not in READ_OPERATIONS:
                raise ParseError("aliased mutation")
    if indirect_depth:
        raise ParseError("unterminated template expression")
    return calls


def _tool_call(
    tokens: list[Token], index: int, *, indirect: bool = False
) -> Call | None:
    cursor = index + 1
    optional = False
    if cursor < len(tokens) and tokens[cursor].value == "?":
        if cursor + 1 >= len(tokens) or tokens[cursor + 1].value != ".":
            return None
        optional = True
        cursor += 2
    elif cursor < len(tokens) and tokens[cursor].value == ".":
        cursor += 1
    elif cursor < len(tokens) and tokens[cursor].value == "[":
        return _computed_call(tokens, cursor, None)
    else:
        return None
    if cursor >= len(tokens):
        return None
    if tokens[cursor].value == "[":
        return _computed_call(tokens, cursor, None)
    member = tokens[cursor]
    if member.kind != "identifier":
        return None
    if member.value.startswith(TOOL_PREFIX):
        return _member_call(tokens, member.value, cursor + 1, indirect or optional)
    if member.value != "mcp__codex_apps":
        return None
    cursor += 1
    if cursor >= len(tokens):
        return None
    if tokens[cursor].value == "[":
        return _computed_call(tokens, cursor, TOOL_PREFIX)
    if tokens[cursor].value != "." or cursor + 1 >= len(tokens):
        return None
    member = tokens[cursor + 1]
    if member.kind != "identifier" or not member.value.startswith("github_"):
        return None
    return _member_call(
        tokens, TOOL_PREFIX + member.value, cursor + 2, indirect or optional
    )


def _computed_call(
    tokens: list[Token], open_index: int, prefix: str | None
) -> Call | None:
    close = _matching(tokens, open_index, "[", "]")
    selector = _strip_parentheses(tokens[open_index + 1 : close])
    if len(selector) != 1 or selector[0].kind != "string":
        raise ParseError("dynamic tool selection")
    value = selector[0].value
    tool = value if prefix is None else prefix + value
    if not tool.startswith(TOOL_PREFIX):
        return None
    return _member_call(tokens, tool, close + 1, indirect=True)


def _member_call(
    tokens: list[Token], tool: str, cursor: int, indirect: bool = False
) -> Call | None:
    operation = tool.rsplit("github_", 1)[-1]
    if indirect and operation not in READ_OPERATIONS:
        raise ParseError("indirect mutation")
    if cursor >= len(tokens) or tokens[cursor].value != "(":
        if operation not in READ_OPERATIONS:
            raise ParseError("uninvoked mutation")
        return None
    return Call(tokens, tool, cursor + 1)


def _reflect_call(tokens: list[Token], index: int) -> Call | None:
    cursor = index + 1
    while cursor < len(tokens) and tokens[cursor].value == ")":
        cursor += 1
    if cursor < len(tokens) and tokens[cursor].value == "?":
        cursor += 1
        if cursor >= len(tokens) or tokens[cursor].value != ".":
            return None
        cursor += 1
    if cursor < len(tokens) and tokens[cursor].value == ".":
        if cursor + 1 >= len(tokens) or tokens[cursor + 1].value != "get":
            return None
        cursor += 2
    elif cursor < len(tokens) and tokens[cursor].value == "[":
        if (
            cursor + 2 >= len(tokens)
            or tokens[cursor + 1].value != "get"
            or tokens[cursor + 2].value != "]"
        ):
            return None
        cursor += 3
    else:
        return None
    while cursor < len(tokens) and tokens[cursor].value == ")":
        cursor += 1
    if cursor >= len(tokens) or tokens[cursor].value != "(":
        return None
    close = _matching(tokens, cursor, "(", ")")
    arguments = _arguments(tokens, cursor + 1, close)
    if len(arguments) != 2 or not _is_tools(arguments[0]):
        return None
    selector = _strip_parentheses(arguments[1])
    if len(selector) != 1 or selector[0].kind != "string":
        raise ParseError("dynamic Reflect.get")
    value = selector[0].value
    tool = value if value.startswith(TOOL_PREFIX) else TOOL_PREFIX + value
    if not value.startswith(TOOL_PREFIX) and not value.startswith("github_"):
        return None
    return _member_call(tokens, tool, close + 1, indirect=True)


def _eval_calls(tokens: list[Token], index: int, depth: int) -> list[Call]:
    open_index = _call_open(tokens, index)
    if open_index is None:
        return []
    close = _matching(tokens, open_index, "(", ")")
    arguments = _arguments(tokens, open_index + 1, close)
    if len(arguments) != 1:
        raise ParseError("dynamic eval")
    argument = _strip_parentheses(arguments[0])
    if len(argument) != 1 or argument[0].kind != "string":
        raise ParseError("dynamic eval")
    nested = _calls(tokenize(argument[0].value), depth + 1)
    if any(
        call.tool.rsplit("github_", 1)[-1] not in READ_OPERATIONS for call in nested
    ):
        raise ParseError("indirect mutation")
    return []


def _direct_member(tokens: list[Token], index: int) -> bool:
    values = [token.value for token in tokens]
    return any(
        index >= offset
        and values[index - offset : index]
        in (
            ["tools", "."],
            ["tools", "?", "."],
            ["tools", ".", "mcp__codex_apps", "."],
            ["tools", "?", ".", "mcp__codex_apps", "."],
        )
        for offset in (2, 3, 4, 5)
    )


def _property_key(tokens: list[Token], index: int) -> bool:
    return index + 1 < len(tokens) and tokens[index + 1].value == ":"
