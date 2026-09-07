"""Bounded literal inspection for supported nested GitHub title calls."""

from __future__ import annotations

from .title_nested_parser import (
    ParseError,
    Token,
    arguments,
    matching,
    tokenize,
    value_end,
)

from .titles import issue_title, pr_title, squash_subject


MAX_CODE = 64 * 1024
MAX_DEPTH = 8
KNOWN_PREFIX = "github_"


def forbidden(code: object) -> bool:
    if not isinstance(code, str):
        return _mentions_supported_call(code if isinstance(code, str) else "")
    bounded_code = code.strip()
    if len(bounded_code) > MAX_CODE:
        return _mentions_supported_call(code)
    try:
        return _inspect(tokenize(bounded_code), 0)
    except ParseError:
        return _mentions_supported_call(code)


def _inspect(tokens: list[Token], depth: int) -> bool:
    if depth > MAX_DEPTH:
        raise ParseError("nested evaluation depth")
    for index, token in enumerate(tokens):
        call = _call_at(tokens, index)
        if call is not None and _call_forbidden(tokens, *call):
            return True
        if token.kind == "identifier" and token.value == "eval":
            nested = _eval_source(tokens, index)
            if nested is not None and _inspect(tokenize(nested), depth + 1):
                return True
        if token.kind == "template" and token.value:
            if _inspect(tokenize(token.value), depth + 1):
                return True
    return False


def _call_forbidden(tokens: list[Token], tool: str, open_index: int) -> bool:
    close = matching(tokens, open_index, "(", ")")
    call_arguments = arguments(tokens, open_index + 1, close)
    if len(call_arguments) != 1 or not call_arguments[0]:
        return True
    fields = _object_fields(call_arguments[0])
    if fields is None:
        return True
    operation = tool.removeprefix("mcp__codex_apps__").removeprefix("github_")
    if operation in {"create_issue", "create_pull_request"}:
        value = fields.get("title", _MISSING)
        predicate = issue_title if operation == "create_issue" else pr_title
        return not isinstance(value, str) or not predicate(value)
    if operation in {"update_issue", "update_pull_request"}:
        value = fields.get("title", _MISSING)
        if value is _MISSING or value is None:
            return False
        predicate = issue_title if operation == "update_issue" else pr_title
        return not isinstance(value, str) or not predicate(value)
    if operation == "merge_pull_request":
        method = fields.get("merge_method")
        if method == "squash":
            number, subject = fields.get("pr_number"), fields.get("commit_title")
            return not squash_subject(subject, number)
        return method is _DYNAMIC
    return False


_MISSING = object()
_DYNAMIC = object()


def _object_fields(tokens: list[Token]) -> dict[str, object] | None:
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
            return _DYNAMIC
    if len(tokens) == 1 and tokens[0].kind == "identifier":
        return _DYNAMIC
    return _DYNAMIC


def _call_at(tokens: list[Token], index: int) -> tuple[str, int] | None:
    token = tokens[index]
    if token.kind != "identifier":
        return None
    if token.value == "tools":
        return _tools_call(tokens, index)
    if token.value == "github":
        return _dot_member(tokens, index, "github_")
    if (
        token.value.startswith("github_")
        and index + 1 < len(tokens)
        and tokens[index + 1].value == "("
    ):
        return token.value, index + 1
    return None


def _tools_call(tokens: list[Token], index: int) -> tuple[str, int] | None:
    cursor = index + 1
    if cursor >= len(tokens) or tokens[cursor].value != ".":
        return None
    member = tokens[cursor + 1] if cursor + 1 < len(tokens) else None
    if member is None:
        return None
    if member.kind == "identifier" and member.value.startswith(
        "mcp__codex_apps__github_"
    ):
        return _member_call(tokens, member.value, cursor + 2)
    if member.kind == "identifier" and member.value == "mcp__codex_apps":
        cursor += 2
        if cursor >= len(tokens):
            return None
        if tokens[cursor].value == ".":
            member = tokens[cursor + 1] if cursor + 1 < len(tokens) else None
            if (
                member is not None
                and member.kind == "identifier"
                and member.value.startswith("github_")
            ):
                return _member_call(
                    tokens, "mcp__codex_apps__" + member.value, cursor + 2
                )
        if (
            tokens[cursor].value == "["
            and cursor + 2 < len(tokens)
            and tokens[cursor + 1].kind == "string"
            and tokens[cursor + 2].value == "]"
        ):
            value = tokens[cursor + 1].value
            if value.startswith("github_"):
                return _member_call(tokens, "mcp__codex_apps__" + value, cursor + 3)
    if (
        member is not None
        and member.kind == "identifier"
        and member.value.startswith("github_")
    ):
        return _member_call(tokens, member.value, cursor + 2)
    return None


def _dot_member(tokens: list[Token], index: int, prefix: str) -> tuple[str, int] | None:
    if index + 2 >= len(tokens) or tokens[index + 1].value != ".":
        return None
    member = tokens[index + 2]
    return (
        _member_call(tokens, member.value, index + 3)
        if member.kind == "identifier" and member.value.startswith(prefix)
        else None
    )


def _member_call(tokens: list[Token], tool: str, index: int) -> tuple[str, int] | None:
    return (tool, index) if index < len(tokens) and tokens[index].value == "(" else None


def _eval_source(tokens: list[Token], index: int) -> str | None:
    if index + 2 >= len(tokens) or tokens[index + 1].value != "(":
        return None
    close = matching(tokens, index + 1, "(", ")")
    eval_arguments = arguments(tokens, index + 2, close)
    return (
        eval_arguments[0][0].value
        if len(eval_arguments) == 1
        and len(eval_arguments[0]) == 1
        and eval_arguments[0][0].kind == "string"
        else None
    )


def _mentions_supported_call(source: str) -> bool:
    try:
        tokens = tokenize(source, strict=False)
    except ParseError:
        return False
    return any(
        _supported_token(tokens, index)
        or token.kind == "template"
        and bool(token.value)
        and _mentions_supported_call(token.value)
        for index, token in enumerate(tokens)
    )


def _supported_token(tokens: list[Token], index: int) -> bool:
    call = _call_at(tokens, index)
    return call is not None and call[0].removeprefix("mcp__codex_apps__") in _SUPPORTED


_SUPPORTED = frozenset(
    {
        "github_create_issue",
        "github_update_issue",
        "github_create_pull_request",
        "github_update_pull_request",
        "github_merge_pull_request",
    }
)
