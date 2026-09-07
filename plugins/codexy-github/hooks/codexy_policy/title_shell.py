"""Title-only inspection for literal GitHub CLI and API shell commands."""

from __future__ import annotations

import re
from collections.abc import Sequence

from .shell_segments import segments
from .titles import issue_title, pr_title, squash_subject


_ASSIGNMENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
_DYNAMIC = re.compile(r"[$`]|__codexy_(?:command|process)_substitution__")
_FIELD_FLAGS = {"-f", "-F", "--field", "--raw-field"}


def forbidden(command: object) -> bool:
    if not isinstance(command, str):
        return False
    parsed = segments(command)
    if parsed is None:
        return bool(
            re.search(r"\bgh\s+(?:issue|pr)\s+(?:create|edit|merge)\b", command)
        )
    return any(_inspect_segment(segment) for segment in parsed)


def _inspect_segment(segment: tuple[str, ...]) -> bool:
    tokens = _command_tokens(segment)
    if not tokens or tokens[0].rsplit("/", 1)[-1] != "gh":
        return False
    args = list(tokens[1:])
    if len(args) < 2:
        return False
    if args[:2] == ["api", "graphql"]:
        return _inspect_graphql(args[2:])
    if args[0] == "api":
        return _inspect_api(args[1:])
    if args[:2] in (["issue", "create"], ["pr", "create"]):
        return _inspect_form(args[0], True, args[2:])
    if args[:2] in (["issue", "edit"], ["pr", "edit"]):
        return _inspect_form(args[0], False, args[2:])
    if args[:2] == ["pr", "merge"]:
        return _inspect_merge(args[2:])
    return False


def _inspect_form(kind: str, create: bool, args: list[str]) -> bool:
    present, value = _option(args, ("--title", "-t"))
    if not present:
        return create
    return not isinstance(value, str) or not (
        issue_title if kind == "issue" else pr_title
    )(value)


def _inspect_merge(args: list[str]) -> bool:
    if "--squash" not in args:
        return False
    present, value = _option(args, ("--subject",))
    if not present:
        return True
    number = next((token for token in args if token.isdigit() and int(token) > 0), None)
    return (
        not isinstance(value, str)
        or number is None
        or not squash_subject(value, int(number))
    )


def _inspect_api(args: list[str]) -> bool:
    method = "GET"
    endpoint = None
    fields: list[str | None] = []
    index = 0
    while index < len(args):
        token = args[index]
        if token in {"--method", "-X"}:
            if index + 1 >= len(args):
                return False
            method, index = args[index + 1].upper(), index + 2
            continue
        if token.startswith("--method="):
            method = token.split("=", 1)[1].upper()
            index += 1
            continue
        if token.startswith("-X") and len(token) > 2:
            method, index = token[2:].upper(), index + 1
            continue
        if token in _FIELD_FLAGS:
            fields.append(args[index + 1] if index + 1 < len(args) else None)
            index += 2
            continue
        if any(token.startswith(flag + "=") for flag in _FIELD_FLAGS):
            fields.append(token.split("=", 1)[1])
            index += 1
            continue
        if endpoint is None and not token.startswith("-"):
            endpoint = token
        index += 1
    if endpoint is None:
        return False
    if endpoint == "graphql":
        return _inspect_graphql(fields)
    if method not in {"POST", "PATCH", "PUT"}:
        return False
    operation = _api_operation(endpoint, method)
    if operation is None:
        return False
    kind, create = operation
    present, value = _field(fields, "title")
    if not present:
        return create
    predicate = issue_title if kind == "issue" else pr_title
    return not isinstance(value, str) or not predicate(value)


def _inspect_graphql(fields: Sequence[str | None]) -> bool:
    present, query = _field(fields, "query")
    if not present:
        return False
    if not isinstance(query, str):
        return True
    match = re.search(
        r"\b(createIssue|updateIssue|createPullRequest|updatePullRequest)\b", query
    )
    if match is None:
        return False
    create = match.group(1).startswith("create")
    kind = "issue" if "Issue" in match.group(1) else "pr"
    title_field = re.search(r"\btitle\s*:", query)
    if title_field is None:
        return create
    title = re.search(r"\btitle\s*:\s*([\"'])(.*?)\1", query, re.S)
    if title is None:
        return True
    predicate = issue_title if kind == "issue" else pr_title
    return not predicate(title.group(2))


def _api_operation(endpoint: str, method: str) -> tuple[str, bool] | None:
    parts = endpoint.strip("/").split("/")
    if len(parts) < 4 or parts[0] != "repos":
        return None
    resource = parts[3]
    if resource not in {"issues", "pulls"}:
        return None
    kind = "issue" if resource == "issues" else "pr"
    if method == "POST" and len(parts) == 4:
        return kind, True
    if method in {"PATCH", "PUT"} and len(parts) == 5 and parts[4].isdigit():
        return kind, False
    return None


def _field(fields: Sequence[str | None], name: str) -> tuple[bool, str | None]:
    values = [
        value
        for value in fields
        if isinstance(value, str) and value.startswith(name + "=")
    ]
    if not values:
        return False, None
    if len(values) != 1:
        return True, None
    value = values[0].split("=", 1)[1]
    return True, None if _DYNAMIC.search(value) else value


def _option(args: list[str], names: tuple[str, ...]) -> tuple[bool, str | None]:
    found: str | None = None
    present = False
    for index, token in enumerate(args):
        if token in names:
            if present or index + 1 >= len(args):
                return True, None
            present = True
            found = args[index + 1]
        elif any(token.startswith(name + "=") for name in names):
            if present:
                return True, None
            present, found = True, token.split("=", 1)[1]
        elif token.startswith("-t") and "-t" in names and len(token) > 2:
            if present:
                return True, None
            present, found = True, token[2:].removeprefix("=")
    return present, None if found is None or _DYNAMIC.search(found) else found


def _command_tokens(segment: tuple[str, ...]) -> tuple[str, ...]:
    index = 0
    while index < len(segment):
        token = segment[index]
        if token == "!" or _ASSIGNMENT.match(token):
            index += 1
            continue
        name = token.rsplit("/", 1)[-1]
        if name in {"command", "exec", "nohup", "nice", "time", "sudo", "timeout"}:
            index += 1
            if index < len(segment) and segment[index] == "--":
                index += 1
            continue
        if name == "env":
            index += 1
            while index < len(segment) and (
                _ASSIGNMENT.match(segment[index]) or segment[index].startswith("-")
            ):
                index += 1
            continue
        break
    return segment[index:]
