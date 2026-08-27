"""Pull-request mutation checks and the direct connector launcher adapter."""

from __future__ import annotations

from typing import Any

from .envelope import Request
from .github_target import (
    graph_bound,
    graph_bound_list,
    graph_common,
    graph_id,
    graph_keys,
    graph_literal,
    graph_nullable,
    graph_object,
)
from .merge import positive_int
from .titles import pr_title

CREATE_FIELDS = set(
    "title body base head draft maintainer_can_modify head_repo".split()
)
UPDATE_FIELDS = set("title body base maintainer_can_modify".split())
REVIEW_ENTRIES = set("body path position line side start_line start_side".split())
GRAPH_REVIEW_FIELDS = set(
    "pullRequestId event action body review commitId comments fileComments clientMutationId".split()
)


def forbidden(request: Request) -> bool:
    if not isinstance(request.tool_input, dict):
        return True
    from .connector import connector_admitted

    return not connector_admitted(request.tool, request.tool_input, request.cwd)


def create(payload: dict[str, Any]) -> bool:
    return (
        set(payload) <= CREATE_FIELDS
        and _nonempty(payload.get("title"))
        and _nonempty(payload.get("base"))
        and _nonempty(payload.get("head"))
        and pr_title(payload["title"])
        and _optional(payload, CREATE_FIELDS)
    )


def metadata(payload: dict[str, Any]) -> bool:
    if not payload or set(payload) - UPDATE_FIELDS:
        return False
    if (
        "title" in payload
        and payload["title"] is not None
        and not pr_title(payload["title"])
    ):
        return False
    if (
        "body" in payload
        and payload["body"] is not None
        and not isinstance(payload["body"], str)
    ):
        return False
    return (
        "base" not in payload or payload["base"] is None or _nonempty(payload["base"])
    )


def state(payload: dict[str, Any]) -> bool:
    return set(payload) == {"state"} and payload["state"] in {"open", "closed"}


def comment(payload: dict[str, Any]) -> bool:
    return set(payload) == {"comment"} and isinstance(payload["comment"], str)


def reviewers(payload: dict[str, Any]) -> bool:
    return (
        set(payload) <= {"reviewers", "team_reviewers"}
        and bool(payload)
        and all(value is None or _strings(value) for value in payload.values())
        and any(_strings(value) for value in payload.values())
    )


def transition(payload: dict[str, Any]) -> bool:
    return set(payload) == {"transition"} and payload["transition"] in {
        "draft",
        "ready",
    }


def review(payload: dict[str, Any]) -> bool:
    if (
        set(payload) - {"action", "body", "commit_id", "file_comments"}
        or payload.get("action") not in {"COMMENT", "APPROVE", "REQUEST_CHANGES"}
        or payload.get("action") in {"COMMENT", "REQUEST_CHANGES"}
        and not _nonempty(payload.get("body"))
        or "body" in payload
        and payload["body"] is not None
        and not isinstance(payload["body"], str)
        or "commit_id" in payload
        and not _nonempty(payload["commit_id"])
    ):
        return False
    return (
        "file_comments" not in payload
        or payload["file_comments"] is None
        or entries(payload["file_comments"])
    )


def _optional(payload: dict[str, Any], allowed: set[str]) -> bool:
    if set(payload) - allowed:
        return False
    for key, value in payload.items():
        if (
            (key == "body" and not (isinstance(value, str) or value is None))
            or (
                key in {"draft", "maintainer_can_modify"}
                and value is not None
                and type(value) is not bool
            )
            or (
                key in {"base", "head", "head_repo"}
                and value is not None
                and not _nonempty(value)
            )
        ):
            return False
    return True


def entries(value: object) -> bool:
    return isinstance(value, list) and all(
        isinstance(item, dict)
        and not set(item) - REVIEW_ENTRIES
        and _nonempty(item.get("body"))
        and _nonempty(item.get("path"))
        and all(_line(item.get(key)) for key in ("position", "line", "start_line"))
        and all(
            item.get(key) is None or _nonempty(item.get(key))
            for key in ("side", "start_side")
        )
        for item in value
    )


def _line(value: object) -> bool:
    return value is None or positive_int(value)


def graphql_review(payload: dict[str, object], transport: dict[str, str]) -> bool:
    if not graph_common(
        payload, GRAPH_REVIEW_FIELDS, {"pullRequestId"}
    ) or not graph_id(payload, "pullRequestId", transport):
        return False
    actions = [key for key in ("event", "action") if key in payload]
    if len(actions) != 1 or payload[actions[0]] not in {
        "COMMENT",
        "APPROVE",
        "REQUEST_CHANGES",
    }:
        return False
    if payload[actions[0]] in {"COMMENT", "REQUEST_CHANGES"} and not any(
        graph_literal(payload.get(key)) for key in {"body", "review"}
    ):
        return False
    if {"body", "review"} <= set(payload) or any(
        key in payload and not graph_nullable(payload[key])
        for key in {"body", "review"}
    ):
        return False
    if (
        "commitId" in payload
        and payload["commitId"] != "null"
        and not graph_id(payload, "commitId", transport)
    ):
        return False
    return all(
        key not in payload or payload[key] == "null" or _graphql_entries(payload[key])
        for key in {"comments", "fileComments"}
    )


def graphql_reviewers(
    payload: dict[str, object], fields: set[str], transport: dict[str, str]
) -> bool:
    return (
        graph_common(
            payload,
            {"pullRequestId", *fields, "union", "clientMutationId"},
            {"pullRequestId"},
        )
        and graph_id(payload, "pullRequestId", transport)
        and all(
            payload[key] == "null"
            or graph_bound_list(payload[key], transport, *graph_keys(key))
            for key in fields
            if key in payload
        )
        and any(
            key in payload
            and graph_bound_list(payload[key], transport, *graph_keys(key))
            for key in fields
        )
        and ("union" not in payload or payload["union"] in {"true", "false"})
    )


def _graphql_entries(value: object) -> bool:
    if not isinstance(value, tuple) or len(value) != 2 or value[0] != "list":
        return False
    return all(
        (item := graph_object(entry)) is not None
        and set(item) <= REVIEW_ENTRIES
        and graph_literal(item.get("body"))
        and graph_literal(item.get("path"))
        and all(_graphql_entry_value(item, key) for key in set(item) - {"body", "path"})
        for entry in value[1]
    )


def _graphql_entry_value(item: dict[str, object], key: str) -> bool:
    value = item[key]
    return (
        value == "null"
        or (key in {"position", "line", "start_line"} and value == "<number>")
        or (key in {"side", "start_side"} and value not in {"<number>", "..."})
    )


def _nonempty(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _strings(value: object) -> bool:
    return isinstance(value, list) and all(_nonempty(item) for item in value)
