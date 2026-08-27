"""Classify mutating GitHub API calls against the owned repository."""

from __future__ import annotations

import json
import re

from .github_target import UnsafeQueryFile, parse_api_args
from .graphql import admitted as graphql_admitted, input_query
from .repository import (
    github_identity,
    read_text,
    repository_identity,
    repository_policy_status,
)
from .repository_pull_request import entries

REPOSITORY = re.compile(r"^/?repos/([^/]+)/([^/]+)(?:/|$)", re.IGNORECASE)


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
        parsed = parse_api_args(args, cwd, read_text)
    except UnsafeQueryFile:
        return True
    if parsed is None:
        return True
    endpoint, method, fields, input_file = parsed
    if endpoint.casefold().strip("/") == "graphql":
        query = (
            input_query(cwd, input_file)
            if input_file is not None
            else fields.get("query")
        )
        return query is None or not graphql_admitted(
            query, {} if input_file else fields, owned_identity
        )
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
    selected = (
        github_identity(f"{match.group(1)}/{match.group(2)}")
        if match is not None
        else None
    )
    if (
        match is None
        or selected is None
        or owned_identity is None
        or selected != owned_identity
    ):
        return False
    path = [part for part in endpoint.strip("/").split("/") if part]
    if len(path) < 4 or path[:3] != ["repos", match.group(1), match.group(2)]:
        return False
    tail = path[3:]
    if tail == ["issues"] and method == "POST":
        return _issue_create(fields)
    if tail[:1] == ["issues"] and len(tail) == 2 and _number(tail[1]):
        return _issue_patch(method, fields)
    if len(tail) == 3 and tail[0] == "issues" and _number(tail[1]):
        if tail[2] == "comments":
            return method == "POST" and set(fields) == {"body"}
        if tail[2] == "labels":
            return method == "POST" and _list_field(fields, "labels")
        if tail[2] == "assignees":
            return method in {"POST", "DELETE"} and _list_field(fields, "assignees")
    if (
        len(tail) == 4
        and tail[:3] == ["issues", tail[1], "labels"]
        and _number(tail[1])
    ):
        return method == "DELETE" and not fields and bool(tail[3])
    if tail == ["pulls"] and method == "POST":
        return _pr_create(fields)
    if tail[:1] == ["pulls"] and len(tail) == 2 and _number(tail[1]):
        return _pr_patch(method, fields)
    if len(tail) == 3 and tail[0] == "pulls" and _number(tail[1]):
        if tail[2] == "reviews":
            return method == "POST" and _review(fields)
        if tail[2] == "requested_reviewers":
            return (
                method in {"POST", "DELETE"}
                and set(fields)
                <= {
                    "reviewers",
                    "team_reviewers",
                }
                and bool(fields)
                and all(_list_field(fields, key) for key in fields)
            )
    return False


def _issue_patch(method: str, fields: dict[str, str]) -> bool:
    if method != "PATCH" or not fields:
        return False
    keys = set(fields)
    if keys <= {"title", "body"}:
        return ("title" in keys and bool(fields["title"])) or "body" in keys
    if keys <= {"state", "state_reason", "duplicate_issue_id"} and fields.get(
        "state"
    ) in {
        "open",
        "closed",
    }:
        reason = fields.get("state_reason")
        if fields["state"] == "open":
            return reason in {None, "reopened"} and "duplicate_issue_id" not in fields
        if reason == "duplicate":
            return keys == {"state", "state_reason", "duplicate_issue_id"} and _number(
                fields["duplicate_issue_id"]
            )
        return (
            reason in {None, "completed", "not_planned"}
            and "duplicate_issue_id" not in fields
        )
    if keys in ({"labels"}, {"assignees"}):
        return _list_field(fields, next(iter(keys)), allow_empty=True)
    return keys == {"milestone"} and _milestone(fields["milestone"])


def _pr_patch(method: str, fields: dict[str, str]) -> bool:
    if method != "PATCH" or not fields:
        return False
    if set(fields) == {"state"}:
        return fields["state"] in {"open", "closed"}
    return set(fields) <= {"title", "body", "base", "maintainer_can_modify"}


def _issue_create(fields: dict[str, str]) -> bool:
    allowed = {"title", "body", "assignees", "milestone", "labels"}
    return (
        set(fields) <= allowed
        and bool(fields.get("title"))
        and all(
            _list_field(fields, key, allow_empty=True)
            for key in ("labels", "assignees")
            if key in fields
        )
        and ("milestone" not in fields or _milestone(fields["milestone"]))
    )


def _pr_create(fields: dict[str, str]) -> bool:
    required, allowed = (
        {"title", "head", "base"},
        {
            "title",
            "head",
            "base",
            "body",
            "draft",
            "maintainer_can_modify",
            "head_repo",
        },
    )
    return (
        set(fields) <= allowed
        and required <= set(fields)
        and all(bool(fields[key]) for key in required)
        and all(
            _boolean(fields[key])
            for key in ("draft", "maintainer_can_modify")
            if key in fields
        )
        and ("head_repo" not in fields or bool(fields["head_repo"]))
    )


def _review(fields: dict[str, str]) -> bool:
    actions = [key for key in ("event", "action") if key in fields]
    if len(actions) != 1 or fields[actions[0]] not in {
        "COMMENT",
        "APPROVE",
        "REQUEST_CHANGES",
    }:
        return False
    if fields[actions[0]] in {"COMMENT", "REQUEST_CHANGES"} and not fields.get("body"):
        return False
    allowed = {"event", "action", "body", "commit_id", "comments"}
    return set(fields) <= allowed and _review_fields(fields)


def _review_fields(fields: dict[str, str]) -> bool:
    if "commit_id" in fields and not fields["commit_id"]:
        return False
    if "comments" not in fields:
        return True
    try:
        comments = json.loads(fields["comments"])
    except json.JSONDecodeError:
        return False
    return entries(comments)


def _list_field(fields: dict[str, str], key: str, *, allow_empty: bool = False) -> bool:
    return set(fields) == {key} and _list_value(fields[key], allow_empty=allow_empty)


def _list_value(value: str, *, allow_empty: bool = False) -> bool:
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError:
        return False
    return (
        isinstance(parsed, list)
        and (allow_empty or bool(parsed))
        and all(isinstance(item, str) and bool(item.strip()) for item in parsed)
    )


def _boolean(value: str) -> bool:
    return value in {"true", "false"}


def _milestone(value: str) -> bool:
    return value == "null" or _number(value)


def _number(value: object) -> bool:
    return (
        isinstance(value, str)
        and value.isascii()
        and value.isdigit()
        and int(value) > 0
    )
