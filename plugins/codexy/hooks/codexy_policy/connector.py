"""Typed GitHub connector ownership and mutation-input admission."""

from __future__ import annotations

from typing import Any

from .github import BodyEvidence, BodySource, Mutation, MutationKind, admitted
from .merge import positive_int
from .repository import OWNED

FIELDS = {
    "create_issue": {"assignees", "body", "labels", "milestone", "repository_full_name", "title"},
    "update_issue": {"assignees", "body", "issue_number", "labels", "milestone", "repository_full_name", "state", "state_reason", "title"},
    "create_pull_request": {"base", "base_branch", "body", "draft", "head", "head_branch", "head_repo", "issue", "maintainer_can_modify", "repository_full_name", "title"},
    "update_pull_request": {"base_branch", "body", "maintainer_can_modify", "pr_number", "repository_full_name", "state", "title"},
    "merge_pull_request": {"commit_message", "commit_title", "expected_head_sha", "merge_method", "pr_number", "repository_full_name"},
    "enable_auto_merge": {"pr_number", "repository_full_name"},
}


def connector_admitted(tool: str, data: dict[str, Any]) -> bool:
    operation = tool.rsplit("github_", 1)[-1]
    owned = _owned(operation, data)
    if owned is None:
        return False
    if not owned:
        return True
    if operation == "create_issue":
        mutation = _connector(MutationKind.ISSUE_CREATE, data, require_title=True, require_body=True)
    elif operation == "update_issue":
        mutation = _connector(MutationKind.ISSUE_UPDATE, data, number="issue_number")
    elif operation == "create_pull_request":
        mutation = _connector(MutationKind.PR_CREATE, data, require_title=True, require_body=True, issue=True)
    elif operation == "update_pull_request":
        mutation = _connector(MutationKind.PR_UPDATE, data, number="pr_number")
    elif operation in {"merge_pull_request", "enable_auto_merge"}:
        mutation = _connector(MutationKind.PR_MERGE, data, number="pr_number")
    else:
        return False
    return mutation is not None and admitted(mutation)


def _owned(operation: str, data: dict[str, Any]) -> bool | None:
    fields = FIELDS.get(operation)
    repository = data.get("repository_full_name")
    if fields is None or set(data).difference(fields) or not isinstance(repository, str):
        return None
    identity = _repository_identity(repository)
    return identity == OWNED if identity is not None else None


def _repository_identity(value: str) -> tuple[str, str, str] | None:
    owner, separator, repository = value.partition("/")
    if (
        separator != "/"
        or "/" in repository
        or not _owner(owner)
        or not _repository(repository)
    ):
        return None
    return "github.com", owner.casefold(), repository.casefold()


def _owner(value: str) -> bool:
    return bool(value) and value[0].isascii() and value[0].isalnum() and all(
        character.isascii() and (character.isalnum() or character == "-")
        for character in value
    )


def _repository(value: str) -> bool:
    return bool(value) and all(
        character.isascii()
        and (character.isalnum() or character in "._-")
        for character in value
    )


def _connector(kind: MutationKind, data: dict[str, Any], *, number: str | None = None, require_title: bool = False, require_body: bool = False, issue: bool = False) -> Mutation | None:
    value = data.get(number) if number is not None else None
    if number is not None and not positive_int(value):
        return None
    title = data.get("title")
    body = data.get("body")
    if (require_title or "title" in data) and not isinstance(title, str):
        return None
    if (require_body or "body" in data) and not isinstance(body, str):
        return None
    linked = data.get("issue") if issue else None
    if linked is not None and not positive_int(linked):
        return None
    return Mutation(kind, True, int(value) if positive_int(value) else None, title, BodyEvidence(body, BodySource.INLINE) if isinstance(body, str) else None, int(linked) if positive_int(linked) else None)
