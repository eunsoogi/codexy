"""Typed PR create/update admission and the normalized operation matrix."""

from __future__ import annotations

import re
from typing import Any

from .body import has_sections
from .github_mutation import Mutation, MutationKind
from .merge import positive_int
from . import repository_pull_request as pr
from .titles import issue_title, pr_title

REQUIRED_SECTIONS = {
    "## Summary",
    "## Rationale",
    "## Changed Areas",
    "## Verification",
    "## Evidence",
    "## Not Run",
    "## Follow-ups",
}
CLOSING = re.compile(
    r"\b(?:close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved)\s+#([1-9][0-9]*)\b",
    re.IGNORECASE,
)


def create(data: dict[str, Any]) -> bool:
    issue = data.get("issue")
    return _valid(
        data.get("title"), data.get("body"), issue if positive_int(issue) else None
    ) and (issue is None or positive_int(issue))


def update(data: dict[str, Any]) -> bool:
    if not positive_int(data.get("pr_number")):
        return False
    if "title" in data and not pr_title(data["title"]):
        return False
    return "body" not in data or _body(data["body"], None)


def shell_create(title: object, body: object) -> bool:
    return _valid(title, body, None)


def shell_update(
    number: object, title: object | None, body: object | None, body_present: bool
) -> bool:
    data: dict[str, Any] = {"pr_number": number}
    if title is not None:
        data["title"] = title
    if body_present:
        data["body"] = body
    return update(data)


def _valid(title: object, body: object, issue: int | None) -> bool:
    return pr_title(title) and _body(body, issue)


def _body(value: object, issue: int | None) -> bool:
    if not has_sections(value, REQUIRED_SECTIONS):
        return False
    assert isinstance(value, str)
    references = [int(number) for number in CLOSING.findall(value)]
    final = next((line for line in reversed(value.splitlines()) if line.strip()), "")
    if issue is not None:
        return references == [issue] and final == f"Fixes #{issue}"
    return not references or (
        len(references) == 1 and final == f"Fixes #{references[0]}"
    )


def payload_eligible(mutation: Mutation) -> bool:
    if mutation.kind == MutationKind.PR_MERGE or (
        mutation.number is not None and mutation.number < 1
    ):
        return False
    payload = mutation.payload or {}
    operation = mutation.operation or _row(mutation.kind, payload)
    if mutation.kind == MutationKind.ISSUE_CREATE:
        return operation == "issue.create" and _issue_create(payload)
    if mutation.kind == MutationKind.PR_CREATE:
        return operation == "pull_request.create" and pr.create(payload)
    if mutation.number is None:
        return False
    if operation == "issue.update_metadata":
        return (
            bool(payload)
            and set(payload) <= {"title", "body"}
            and _issue_metadata(payload)
        )
    if operation == "issue.set_state":
        return _issue_state(payload)
    if operation in {
        "issue.set_labels",
        "issue.set_assignees",
        "issue.add_labels",
        "issue.remove_label",
        "issue.add_assignees",
        "issue.remove_assignees",
    }:
        return _issue_labels(operation, payload)
    if operation == "issue.comment":
        return set(payload) == {"comment"} and isinstance(payload["comment"], str)
    if operation == "issue.set_milestone":
        return set(payload) == {"milestone"} and _milestone(payload["milestone"])
    if operation == "pull_request.update_metadata":
        return (
            bool(payload)
            and set(payload)
            <= {
                "title",
                "body",
                "base",
                "maintainer_can_modify",
            }
            and pr.metadata(payload)
        )
    if operation == "pull_request.set_state":
        return pr.state(payload)
    if operation == "pull_request.comment":
        return pr.comment(payload)
    if operation == "pull_request.submit_review":
        return pr.review(payload)
    if operation == "pull_request.set_reviewers":
        return pr.reviewers(payload)
    if operation in {"pull_request.convert_to_draft", "pull_request.mark_ready"}:
        return pr.transition(payload)
    return False


def _row(kind: MutationKind, payload: dict[str, Any]) -> str | None:
    if kind == MutationKind.ISSUE_CREATE:
        return "issue.create"
    if kind == MutationKind.PR_CREATE:
        return "pull_request.create"
    keys = set(payload)
    if kind == MutationKind.ISSUE_UPDATE:
        if keys and keys <= {"title", "body"}:
            return "issue.update_metadata"
        if "state" in keys:
            return "issue.set_state"
        if keys == {"labels"}:
            return "issue.set_labels"
        if keys == {"assignees"}:
            return "issue.set_assignees"
        if keys == {"milestone"}:
            return "issue.set_milestone"
    if kind == MutationKind.PR_UPDATE:
        if keys == {"state"}:
            return "pull_request.set_state"
        if keys & {"reviewers", "team_reviewers"}:
            return "pull_request.set_reviewers"
        if keys == {"comment"}:
            return "pull_request.comment"
        if keys == {"transition"}:
            return "pull_request.mark_ready"
        if keys:
            return "pull_request.update_metadata"
    return None


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


def _issue_create(payload: dict[str, Any]) -> bool:
    return (
        set(payload) <= {"title", "body", "assignees", "labels", "milestone"}
        and _nonempty(payload.get("title"))
        and issue_title(payload["title"])
        and _optional_fields(payload)
    )


def _issue_metadata(payload: dict[str, Any]) -> bool:
    return (payload.get("title") is None or issue_title(payload["title"])) and (
        payload.get("body") is None or isinstance(payload["body"], str)
    )


def _issue_state(payload: dict[str, Any]) -> bool:
    allowed = {"state", "state_reason", "duplicate_issue_id"}
    if set(payload) - allowed or payload.get("state") not in {"open", "closed"}:
        return False
    reason = payload.get("state_reason")
    if payload["state"] == "open":
        return reason in {None, "reopened"} and "duplicate_issue_id" not in payload
    return (
        reason == "duplicate"
        and set(payload) == allowed
        and positive_int(payload.get("duplicate_issue_id"))
    ) or (
        reason in {None, "completed", "not_planned"}
        and "duplicate_issue_id" not in payload
    )


def _issue_labels(operation: str, payload: dict[str, Any]) -> bool:
    if operation == "issue.remove_label":
        return set(payload) == {"label"} and _nonempty(payload["label"])
    field = "labels" if "label" in operation else "assignees"
    nonempty = operation not in {"issue.set_labels", "issue.set_assignees"}
    return set(payload) == {field} and _strings(payload[field], nonempty=nonempty)


def _optional_fields(payload: dict[str, Any]) -> bool:
    for key, value in payload.items():
        if (
            (key == "body" and not (isinstance(value, str) or value is None))
            or (
                key in {"assignees", "labels"}
                and value is not None
                and not _strings(value, nonempty=False)
            )
            or (key == "milestone" and not _milestone(value))
            or (key == "draft" and type(value) is not bool)
            or (
                key == "maintainer_can_modify"
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
