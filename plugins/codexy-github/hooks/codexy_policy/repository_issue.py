"""Issue-side mutation checks and the direct connector launcher adapter."""

from typing import Any

from .envelope import Request
from .github_target import (
    graph_bound,
    graph_bound_list,
    graph_common,
    graph_id,
    graph_keys,
    graph_list,
    graph_literal,
    graph_nullable,
)
from .merge import positive_int
from .titles import graphql_title, issue_title


def forbidden(request: Request) -> bool:
    if not isinstance(request.tool_input, dict):
        return True
    from .connector import connector_admitted

    return not connector_admitted(request.tool, request.tool_input, request.cwd)


def create(payload: dict[str, Any]) -> bool:
    return (
        set(payload) <= {"title", "body", "assignees", "labels", "milestone"}
        and _nonempty(payload.get("title"))
        and issue_title(payload["title"])
        and _optional(payload)
    )


def metadata(payload: dict[str, Any]) -> bool:
    return (
        "title" not in payload
        or payload["title"] is None
        or issue_title(payload["title"])
    ) and ("body" not in payload or payload["body"] is None or _text(payload["body"]))


def state(payload: dict[str, Any]) -> bool:
    if set(payload) - {"state", "state_reason", "duplicate_issue_id"}:
        return False
    if payload.get("state") not in {"open", "closed"}:
        return False
    reason = payload.get("state_reason")
    if payload["state"] == "open":
        return reason in {None, "reopened"} and "duplicate_issue_id" not in payload
    if reason == "duplicate":
        return set(payload) == {
            "state",
            "state_reason",
            "duplicate_issue_id",
        } and positive_int(payload.get("duplicate_issue_id"))
    return (
        reason in {None, "completed", "not_planned"}
        and "duplicate_issue_id" not in payload
    )


def labels(operation: str, payload: dict[str, Any]) -> bool:
    if operation == "issue.remove_label":
        return set(payload) == {"label"} and _nonempty(payload["label"])
    field = "labels" if "label" in operation else "assignees"
    return set(payload) == {field} and _strings(
        payload[field],
        nonempty=operation not in {"issue.set_labels", "issue.set_assignees"},
    )


def comment(payload: dict[str, Any]) -> bool:
    return set(payload) == {"comment"} and _text(payload["comment"])


def milestone(payload: dict[str, Any]) -> bool:
    return set(payload) == {"milestone"} and _milestone(payload["milestone"])


def _optional(payload: dict[str, Any]) -> bool:
    for key, value in payload.items():
        if key == "body" and not (_text(value) or value is None):
            return False
        if (
            key in {"assignees", "labels"}
            and value is not None
            and not _strings(value, nonempty=False)
        ):
            return False
        if key == "milestone" and not _milestone(value):
            return False
    return True


def _text(value: object) -> bool:
    return isinstance(value, str)


def _nonempty(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _strings(value: object, *, nonempty: bool = True) -> bool:
    return (
        isinstance(value, list)
        and (not nonempty or bool(value))
        and all(_nonempty(item) for item in value)
    )


def _milestone(value: object) -> bool:
    return value is None or positive_int(value) or _nonempty(value)


def graphql_issue(
    name: str, payload: dict[str, object], transport: dict[str, str]
) -> bool:
    if name == "createIssue":
        allowed = {
            "repositoryId",
            "title",
            "body",
            "assigneeIds",
            "milestoneId",
            "labelIds",
            "clientMutationId",
        }
        return (
            graph_common(payload, allowed, {"repositoryId", "title"})
            and graph_id(payload, "repositoryId", transport)
            and graphql_title(payload["title"], issue_title)
            and _graphql_optional(payload, allowed, transport)
        )
    if name == "updateIssue":
        return _graphql_update(payload, transport)
    if name == "closeIssue":
        return _graphql_close(payload, transport)
    if name == "reopenIssue":
        return _graphql_identity(payload, "issueId", transport)
    if name == "addComment":
        return (
            graph_common(
                payload,
                {"subjectId", "body", "clientMutationId"},
                {"subjectId", "body"},
            )
            and graph_id(payload, "subjectId", transport)
            and graph_literal(payload["body"])
        )
    if name in {"addLabelsToLabelable", "removeLabelsFromLabelable"}:
        return _graphql_list_action(payload, "labelableId", "labelIds", transport)
    if name in {"addAssigneesToAssignable", "removeAssigneesFromAssignable"}:
        return _graphql_list_action(payload, "assignableId", "assigneeIds", transport)
    return False


def _graphql_optional(
    payload: dict[str, object], allowed: set[str], transport: dict[str, str]
) -> bool:
    if set(payload) - allowed:
        return False
    for key, value in payload.items():
        if key in {"body", "milestoneId"} and not graph_nullable(value):
            return False
        if key.endswith("Ids") and (
            not graph_list(value, allow_empty=True)
            or not graph_bound_list(
                value, transport, *graph_keys(key), allow_empty=True
            )
        ):
            return False
        if (
            key == "milestoneId"
            and value != "null"
            and not graph_id(payload, key, transport)
        ):
            return False
        if (
            key == "clientMutationId"
            and value != "null"
            and not graph_bound(
                value, transport, "client_mutation_id", "clientMutationId"
            )
        ):
            return False
    return True


def _graphql_update(payload: dict[str, object], transport: dict[str, str]) -> bool:
    if not graph_id(payload, "issueId", transport):
        return False
    semantic = set(payload) - {"issueId", "clientMutationId"}
    if semantic and semantic <= {"title", "body"}:
        return all(
            value == "null"
            or (key == "title" and graphql_title(value, issue_title))
            or (key == "body" and graph_literal(value))
            for key, value in ((key, payload[key]) for key in semantic)
        )
    if semantic in ({"labelIds"}, {"assigneeIds"}):
        key = next(iter(semantic))
        return graph_bound_list(
            payload[key], transport, *graph_keys(key), allow_empty=True
        )
    return semantic == {"milestoneId"} and (
        payload["milestoneId"] == "null" or graph_id(payload, "milestoneId", transport)
    )


def _graphql_close(payload: dict[str, object], transport: dict[str, str]) -> bool:
    allowed = {"issueId", "stateReason", "duplicateIssueId", "clientMutationId"}
    if not graph_common(payload, allowed, {"issueId"}) or not graph_id(
        payload, "issueId", transport
    ):
        return False
    reason = payload.get("stateReason")
    if reason == "null":
        reason = None
    duplicate = (
        graph_common(payload, allowed, {"issueId", "stateReason", "duplicateIssueId"})
        and reason == "DUPLICATE"
        and graph_id(payload, "duplicateIssueId", transport)
    )
    normal = (
        reason in {None, "COMPLETED", "NOT_PLANNED"}
        and "duplicateIssueId" not in payload
    )
    return duplicate or normal


def _graphql_list_action(
    payload: dict[str, object], target: str, values: str, transport: dict[str, str]
) -> bool:
    return (
        graph_common(payload, {target, values, "clientMutationId"}, {target, values})
        and graph_id(payload, target, transport)
        and graph_list(payload[values])
        and graph_bound_list(payload[values], transport, *graph_keys(values))
    )


def _graphql_identity(
    payload: dict[str, object], key: str, transport: dict[str, str]
) -> bool:
    return graph_common(payload, {key, "clientMutationId"}, {key}) and graph_id(
        payload, key, transport
    )
