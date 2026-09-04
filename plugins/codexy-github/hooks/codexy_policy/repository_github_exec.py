"""Admission for GitHub connector calls nested inside ``functions.exec``."""

from __future__ import annotations

from .connector import READ_OPERATIONS, connector_admitted
from .envelope import Request
from .repository_github_exec_parser import LiteralParser, ParseError, Token, tokenize

MAX_CODE = 64 * 1024
TOOL_PREFIX = "mcp__codex_apps__github_"
UNINSPECTABLE = "UNINSPECTABLE_USE_DIRECT_ADMITTED_SURFACE"
POLICY = "POLICY_USE_DIRECT_ADMITTED_SURFACE"


def forbidden(request: Request) -> bool | str:
    if not isinstance(request.tool_input, dict):
        return UNINSPECTABLE
    code = request.tool_input.get("code")
    if not isinstance(code, str) or len(code) > MAX_CODE:
        return UNINSPECTABLE
    try:
        tokens = tokenize(code)
        calls = _calls(tokens)
    except ParseError:
        return UNINSPECTABLE
    for tool, index in calls:
        if tool.rsplit("github_", 1)[-1] in READ_OPERATIONS:
            continue
        try:
            parser = LiteralParser(tokens, index)
            payload = parser.parse()
            parser.expect(")")
        except ParseError:
            return UNINSPECTABLE
        if not connector_admitted(tool, payload, request.cwd):
            return POLICY
    return False


def _calls(tokens: list[Token]) -> list[tuple[str, int]]:
    calls: list[tuple[str, int]] = []
    for index, token in enumerate(tokens):
        if token.kind == "identifier" and token.value == "tools":
            _check_computed(tokens, index)
        elif token.kind == "string" and token.value.startswith(TOOL_PREFIX):
            if _reflect_target(tokens, index):
                _check_tool(token.value)
        if token.kind != "identifier":
            continue
        if token.value.startswith(TOOL_PREFIX):
            operation = token.value.rsplit("github_", 1)[-1]
            direct = (
                index >= 2
                and tokens[index - 2].value == "tools"
                and tokens[index - 1].value == "."
            )
            if not direct:
                if operation not in READ_OPERATIONS:
                    raise ParseError("aliased mutation")
            elif index + 1 >= len(tokens) or tokens[index + 1].value != "(":
                if operation not in READ_OPERATIONS:
                    raise ParseError("uninvoked mutation")
            else:
                calls.append((token.value, index + 2))
        elif (
            token.value.startswith("github_") and token.value[7:] not in READ_OPERATIONS
        ):
            raise ParseError("aliased mutation name")
    return calls


def _check_computed(tokens: list[Token], index: int) -> None:
    if index + 1 >= len(tokens):
        return
    if tokens[index + 1].value == "[":
        if index + 2 >= len(tokens) or tokens[index + 2].kind != "string":
            raise ParseError("dynamic tool selection")
        value = tokens[index + 2].value
        if value.startswith(TOOL_PREFIX):
            _check_tool(value)
        return
    if index + 3 < len(tokens) and [tokens[index + n].value for n in range(1, 4)] == [
        ".",
        "mcp__codex_apps",
        "[",
    ]:
        if index + 4 >= len(tokens) or tokens[index + 4].kind != "string":
            raise ParseError("dynamic nested tool selection")
        value = tokens[index + 4].value
        if value.startswith("github_"):
            _check_tool(TOOL_PREFIX + value)


def _check_tool(tool: str) -> None:
    if tool.rsplit("github_", 1)[-1] not in READ_OPERATIONS:
        raise ParseError("computed mutation")


def _reflect_target(tokens: list[Token], index: int) -> bool:
    return index >= 6 and [tokens[index - n].value for n in range(1, 7)] == [
        ",",
        "tools",
        "(",
        "get",
        ".",
        "Reflect",
    ]
