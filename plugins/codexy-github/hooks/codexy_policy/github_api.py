"""Classify mutating GitHub API calls against the owned repository."""

from __future__ import annotations

import json
import re

from .graphql import admitted as graphql_admitted
from .repository import (
    github_identity,
    read_text,
    repository_identity,
    repository_policy_status,
)

TYPED_FIELD_OPTIONS = {"-F", "--field"}
FIELD_OPTIONS = {"-f", "--raw-field"} | TYPED_FIELD_OPTIONS
VALUE_OPTIONS = {"--cache", "--hostname", "--jq", "--preview", "--template"}
HEADER_OPTIONS = {"-H", "--header"}
FLAG_OPTIONS = {"--include", "-i", "--paginate", "--slurp", "--silent", "--verbose"}
REPOSITORY = re.compile(r"^/?repos/([^/]+)/([^/]+)(?:/|$)", re.IGNORECASE)


class _UnsafeQueryFile(Exception):
    pass


def forbidden(
    args: list[str],
    default_owned: bool,
    cwd: str,
    owned_identity: tuple[str, str, str] | None = None,
    policy_status: bool | None = None,
    policy_bound: bool = False,
) -> bool:
    if not policy_bound and owned_identity is None:
        owned_identity = repository_identity(cwd)
    if not policy_bound:
        policy_status = repository_policy_status(cwd)
    if policy_status is None:
        return True
    try:
        parsed = _parse(args, cwd)
    except _UnsafeQueryFile:
        return True
    if parsed is None:
        return True
    endpoint, method, fields, input_file = parsed
    if endpoint.casefold().strip("/") == "graphql":
        if input_file is not None:
            query = _input_query(cwd, input_file)
            return query is None or not graphql_admitted(query, {}, owned_identity)
        query = fields.get("query")
        return query is None or not graphql_admitted(query, fields, owned_identity)
    if method in {"GET", "HEAD"}:
        return False
    return not _rest_allowed(endpoint, method, fields, owned_identity)


def _rest_allowed(
    endpoint: str,
    method: str,
    fields: dict[str, str],
    owned_identity: tuple[str, str, str] | None,
) -> bool:
    match = REPOSITORY.match(endpoint)
    if match is None or github_identity(f"{match.group(1)}/{match.group(2)}") != owned_identity:
        return False
    path = [part for part in endpoint.strip("/").split("/") if part]
    if len(path) < 4 or path[:3] != ["repos", match.group(1), match.group(2)]:
        return False
    tail = path[3:]
    if tail == ["issues"] and method == "POST":
        return _fields(fields, {"title", "body", "assignees", "milestone", "labels"}, {"title"})
    if tail[:1] == ["issues"] and len(tail) == 2 and _number(tail[1]):
        return _issue_patch(method, fields)
    if len(tail) == 3 and tail[0] == "issues" and _number(tail[1]):
        if tail[2] == "comments" and method == "POST":
            return set(fields) == {"body"}
        if tail[2] == "labels":
            return method == "POST" and set(fields) == {"labels"}
        if tail[2] == "assignees":
            return method in {"POST", "DELETE"} and set(fields) == {"assignees"}
    if len(tail) == 4 and tail[0] == "issues" and _number(tail[1]) and tail[2] == "labels":
        return method == "DELETE" and not fields and bool(tail[3])
    if tail == ["pulls"] and method == "POST":
        return _fields(fields, {"title", "head", "base", "body", "draft", "maintainer_can_modify", "head_repo"}, {"title", "head", "base"})
    if tail[:1] == ["pulls"] and len(tail) == 2 and _number(tail[1]):
        return _pr_patch(method, fields)
    if len(tail) == 3 and tail[0] == "pulls" and _number(tail[1]):
        if tail[2] == "reviews" and method == "POST":
            actions = {key for key in ("event", "action") if key in fields}
            return (
                len(actions) == 1
                and next(iter(actions)) in fields
                and fields[next(iter(actions))] in {"COMMENT", "APPROVE", "REQUEST_CHANGES"}
                and _fields(fields, {"event", "action", "body", "commit_id", "comments"}, actions)
            )
        if tail[2] == "requested_reviewers" and method in {"POST", "DELETE"}:
            return set(fields) <= {"reviewers", "team_reviewers"} and bool(fields)
    return False


def _issue_patch(method: str, fields: dict[str, str]) -> bool:
    if method != "PATCH" or not fields:
        return False
    keys = set(fields)
    if keys <= {"title", "body"}:
        return "title" in keys and bool(fields["title"]) or "body" in keys
    if keys <= {"state", "state_reason", "duplicate_issue_id"} and fields.get("state") in {"open", "closed"}:
        reason = fields.get("state_reason")
        if fields["state"] == "open":
            return reason in {None, "reopened"} and "duplicate_issue_id" not in fields
        if reason == "duplicate":
            return keys == {"state", "state_reason", "duplicate_issue_id"} and _number(fields["duplicate_issue_id"])
        return reason in {None, "completed", "not_planned"} and "duplicate_issue_id" not in fields
    if keys in ({"labels"}, {"assignees"}):
        return True
    return keys == {"milestone"} and bool(fields["milestone"] or fields["milestone"] == "null")


def _pr_patch(method: str, fields: dict[str, str]) -> bool:
    if method != "PATCH" or not fields:
        return False
    if set(fields) == {"state"}:
        return fields["state"] in {"open", "closed"}
    return set(fields) <= {"title", "body", "base", "maintainer_can_modify"} and bool(fields)


def _fields(fields: dict[str, str], allowed: set[str], required: set[str]) -> bool:
    return set(fields) <= allowed and required <= set(fields)


def _number(value: object) -> bool:
    return isinstance(value, str) and value.isascii() and value.isdigit() and int(value) > 0


def _parse(
    args: list[str], cwd: str
) -> tuple[str, str, dict[str, str], str | None] | None:
    method, fields, input_file, positionals, index = None, {}, None, [], 0
    while index < len(args):
        token = args[index]
        if token in {"-X", "--method"}:
            if method is not None or index + 1 >= len(args):
                return None
            method, index = args[index + 1].upper(), index + 2
        elif token.startswith("--method=") or token.startswith("-X="):
            if method is not None:
                return None
            method, index = token.split("=", 1)[1].upper(), index + 1
        elif token.startswith("-X") and len(token) > 2:
            if method is not None:
                return None
            method, index = token[2:].upper(), index + 1
        elif token in FIELD_OPTIONS:
            if index + 1 >= len(args) or not _field(
                fields, args[index + 1], cwd if token in TYPED_FIELD_OPTIONS else None
            ):
                return None
            index += 2
        elif any(token.startswith(option + "=") for option in FIELD_OPTIONS):
            typed = any(
                token.startswith(option + "=") for option in TYPED_FIELD_OPTIONS
            )
            if not _field(fields, token.split("=", 1)[1], cwd if typed else None):
                return None
            index += 1
        elif token == "--input":
            if input_file is not None or index + 1 >= len(args):
                return None
            input_file, index = args[index + 1], index + 2
        elif token.startswith("--input="):
            if input_file is not None:
                return None
            input_file, index = token.split("=", 1)[1], index + 1
        elif token in VALUE_OPTIONS:
            if index + 1 >= len(args):
                return None
            index += 2
        elif any(token.startswith(option + "=") for option in VALUE_OPTIONS):
            index += 1
        elif token in HEADER_OPTIONS:
            if index + 1 >= len(args):
                return None
            index += 2
        elif token.startswith("-H") and len(token) > 2:
            index += 1
        elif token.startswith("--header="):
            index += 1
        elif token in FLAG_OPTIONS:
            index += 1
        elif token.startswith("-"):
            return None
        else:
            positionals.append(token)
            index += 1
    if len(positionals) != 1 or not positionals[0]:
        return None
    return (
        positionals[0],
        method or ("POST" if fields or input_file is not None else "GET"),
        fields,
        input_file,
    )


def _field(fields: dict[str, str], value: str, typed_cwd: str | None) -> bool:
    name, separator, content = value.partition("=")
    if not separator or not name or name in fields:
        return False
    if typed_cwd is not None and name == "query" and content.startswith("@"):
        loaded = read_text(typed_cwd, content[1:])
        if loaded is None:
            raise _UnsafeQueryFile
        content = loaded
    fields[name] = content
    return True


def _input_query(cwd: str, target: str) -> str | None:
    content = read_text(cwd, target)
    if content is None:
        return None
    try:
        body = json.loads(content)
    except json.JSONDecodeError:
        return None
    query = body.get("query") if isinstance(body, dict) else None
    return query if isinstance(query, str) else None
