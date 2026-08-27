"""Typed PR create/update admission contract shared by tool and CLI policies."""

from __future__ import annotations

import re
from typing import Any

from .body import has_sections
from .github_mutation import Mutation, MutationKind
from .merge import positive_int
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
    """Classify one normalized closed-matrix operation without granting it."""
    if mutation.kind == MutationKind.PR_MERGE or mutation.number is not None and mutation.number < 1:
        return False
    payload = mutation.payload or {}
    operation = mutation.operation or _row(mutation.kind, payload)
    if mutation.kind == MutationKind.ISSUE_CREATE:
        return operation == "issue.create" and _issue_create(payload)
    if mutation.kind == MutationKind.PR_CREATE:
        return operation == "pull_request.create" and _pr_create(payload)
    if mutation.number is None:
        return False
    if operation == "issue.update_metadata":
        return bool(payload) and set(payload) <= {"title", "body"} and _issue_metadata(payload)
    if operation == "issue.set_state":
        return _issue_state(payload)
    if operation in {"issue.set_labels", "issue.set_assignees"}:
        field = "labels" if operation.endswith("labels") else "assignees"
        return set(payload) == {field} and _strings(payload[field])
    if operation == "issue.comment":
        return set(payload) == {"comment"} and _text(payload["comment"])
    if operation == "issue.set_milestone":
        return set(payload) == {"milestone"} and _milestone(payload["milestone"])
    if operation == "pull_request.update_metadata":
        return bool(payload) and set(payload) <= {"title", "body", "base", "maintainer_can_modify"} and _pr_metadata(payload)
    if operation == "pull_request.set_state":
        return set(payload) == {"state"} and payload["state"] in {"open", "closed"}
    if operation == "pull_request.comment":
        return set(payload) == {"comment"} and _text(payload["comment"])
    if operation == "pull_request.submit_review":
        return _review_payload(payload)
    if operation == "pull_request.set_reviewers":
        return _reviewers(payload)
    if operation in {"pull_request.convert_to_draft", "pull_request.mark_ready"}:
        return set(payload) == {"transition"} and payload["transition"] in {"draft", "ready"}
    return False


def _row(kind: MutationKind, payload: dict[str, Any]) -> str | None:
    if kind == MutationKind.ISSUE_CREATE:
        return "issue.create"
    if kind == MutationKind.PR_CREATE:
        return "pull_request.create"
    keys = set(payload)
    if kind == MutationKind.ISSUE_UPDATE:
        if keys and keys <= {"title", "body"}: return "issue.update_metadata"
        if "state" in keys: return "issue.set_state"
        if keys == {"labels"}: return "issue.set_labels"
        if keys == {"assignees"}: return "issue.set_assignees"
        if keys == {"milestone"}: return "issue.set_milestone"
    if kind == MutationKind.PR_UPDATE:
        if keys == {"state"}: return "pull_request.set_state"
        if keys & {"reviewers", "team_reviewers"}: return "pull_request.set_reviewers"
        if keys == {"comment"}: return "pull_request.comment"
        if keys == {"transition"}: return "pull_request.mark_ready"
        if keys: return "pull_request.update_metadata"
    return None


def _text(value: object) -> bool:
    return isinstance(value, str)


def _nonempty(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _strings(value: object) -> bool:
    return isinstance(value, list) and all(_nonempty(item) for item in value)


def _milestone(value: object) -> bool:
    return value is None or positive_int(value) or _nonempty(value)


def _issue_create(payload: dict[str, Any]) -> bool:
    return (
        set(payload) <= {"title", "body", "assignees", "labels", "milestone"}
        and _nonempty(payload.get("title"))
        and issue_title(payload["title"])
        and _optional_fields(payload)
    )


def _pr_create(payload: dict[str, Any]) -> bool:
    return (
        set(payload) <= {"title", "body", "base", "head", "draft", "maintainer_can_modify", "head_repo"}
        and _nonempty(payload.get("title"))
        and _nonempty(payload.get("base"))
        and _nonempty(payload.get("head"))
        and _optional_fields(payload)
        and pr_title(payload["title"])
    )


def _issue_metadata(payload: dict[str, Any]) -> bool:
    return ("title" not in payload or payload["title"] is None or issue_title(payload["title"])) and (
        "body" not in payload or payload["body"] is None or _text(payload["body"])
    )


def _pr_metadata(payload: dict[str, Any]) -> bool:
    return (
        ("title" not in payload or payload["title"] is None or pr_title(payload["title"]))
        and ("body" not in payload or payload["body"] is None or _text(payload["body"]))
        and ("base" not in payload or payload["base"] is None or _nonempty(payload["base"]))
        and ("maintainer_can_modify" not in payload or payload["maintainer_can_modify"] is None or type(payload["maintainer_can_modify"]) is bool)
    )


def _optional_fields(payload: dict[str, Any]) -> bool:
    for key, value in payload.items():
        if key == "body" and not (_text(value) or value is None): return False
        if key in {"assignees", "labels"} and value is not None and not _strings(value): return False
        if key == "milestone" and not _milestone(value): return False
        if key == "draft" and type(value) is not bool: return False
        if key == "maintainer_can_modify" and value is not None and type(value) is not bool: return False
        if key in {"base", "head", "head_repo"} and value is not None and not _nonempty(value): return False
    return True


def _issue_state(payload: dict[str, Any]) -> bool:
    if set(payload) - {"state", "state_reason", "duplicate_issue_id"} or payload.get("state") not in {"open", "closed"}:
        return False
    reason = payload.get("state_reason")
    if payload["state"] == "open":
        return reason in {None, "reopened"} and "duplicate_issue_id" not in payload
    if reason == "duplicate":
        return set(payload) == {"state", "state_reason", "duplicate_issue_id"} and positive_int(payload.get("duplicate_issue_id"))
    return reason in {None, "completed", "not_planned"} and "duplicate_issue_id" not in payload


def _reviewers(payload: dict[str, Any]) -> bool:
    return (
        set(payload) <= {"reviewers", "team_reviewers"}
        and bool(payload)
        and all(value is None or _strings(value) for value in payload.values())
        and any(_strings(value) for value in payload.values())
    )


def _review_payload(payload: dict[str, Any]) -> bool:
    allowed = {"action", "body", "commit_id", "file_comments"}
    if set(payload) - allowed or payload.get("action") not in {"COMMENT", "APPROVE", "REQUEST_CHANGES"}:
        return False
    if "body" in payload and payload["body"] is not None and not _text(payload["body"]): return False
    if "commit_id" in payload and not _nonempty(payload["commit_id"]): return False
    return "file_comments" not in payload or payload["file_comments"] is None or _comment_entries(payload["file_comments"])


def _comment_entries(value: object) -> bool:
    allowed = {"body", "path", "position", "line", "side", "start_line", "start_side"}
    return isinstance(value, list) and all(
        isinstance(item, dict)
        and not set(item) - allowed
        and _nonempty(item.get("body"))
        and _nonempty(item.get("path"))
        and all(_line(item.get(key)) for key in ("position", "line", "start_line"))
        and all(item.get(key) is None or _nonempty(item.get(key)) for key in ("side", "start_side"))
        for item in value
    )


def _line(value: object) -> bool:
    return value is None or positive_int(value)
