"""Typed GitHub connector ownership and closed-matrix admission."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from .github import admitted
from .github_mutation import Mutation, MutationKind
from .repository import repository_identity, repository_policy_status

FIELDS = {
    "create_issue": {"assignees", "body", "labels", "milestone", "repository_full_name", "title"},
    "update_issue": {"assignees", "body", "issue_number", "labels", "milestone", "repository_full_name", "state", "state_reason", "title"},
    "create_pull_request": {"base", "base_branch", "body", "draft", "head", "head_branch", "head_repo", "issue", "maintainer_can_modify", "repository_full_name", "title"},
    "update_pull_request": {"base_branch", "body", "maintainer_can_modify", "pr_number", "repository_full_name", "state", "title"},
    "add_comment_to_issue": {"comment", "pr_number", "repo_full_name"},
    "add_issue_assignees": {"assignees", "issue_number", "repository_full_name"},
    "remove_issue_assignees": {"assignees", "issue_number", "repository_full_name"},
    "add_issue_labels": {"issue_number", "labels", "repository_full_name"},
    "remove_issue_label": {"issue_number", "label", "repository_full_name"},
    "add_review_to_pr": {"action", "commit_id", "file_comments", "pr_number", "repo_full_name", "review"},
    "request_pull_request_reviewers": {"pr_number", "repository_full_name", "reviewers", "team_reviewers"},
    "remove_pull_request_reviewers": {"pr_number", "repository_full_name", "reviewers", "team_reviewers"},
    "convert_pull_request_to_draft": {"pr_number", "repository_full_name"},
    "mark_pull_request_ready_for_review": {"pr_number", "repository_full_name"},
    "merge_pull_request": {"commit_message", "commit_title", "expected_head_sha", "merge_method", "pr_number", "repository_full_name"},
    "enable_auto_merge": {"pr_number", "repository_full_name"},
}

READ_OPERATIONS = frozenset({
    "compare_commits", "download_user_content", "download_workflow_artifact", "fetch",
    "fetch_blob", "fetch_commit", "fetch_commit_workflow_runs", "fetch_file", "fetch_issue",
    "fetch_issue_comments", "fetch_pr", "fetch_pr_comments", "fetch_pr_file_patch",
    "fetch_pr_patch", "fetch_workflow_job_logs", "fetch_workflow_job_steps",
    "fetch_workflow_run_artifacts", "fetch_workflow_run_jobs", "get_commit_combined_status",
    "get_issue_comment_reactions", "get_pr_diff", "get_pr_info", "get_pr_reactions",
    "get_pr_review_comment_reactions", "get_profile", "get_repo", "get_repo_collaborator_permission",
    "get_user_login", "get_users_recent_prs_in_repo", "list_installations", "list_installed_accounts",
    "list_pr_changed_filenames", "list_pull_request_review_threads", "list_pull_request_reviews",
    "list_recent_issues", "list_repositories", "list_repositories_by_affiliation",
    "list_repositories_by_installation", "list_user_org_memberships", "list_user_orgs", "search",
    "search_branches", "search_commits", "search_installed_repositories_streaming",
    "search_installed_repositories_v2", "search_issues", "search_prs", "search_repositories",
})


def connector_admitted(tool: str, data: dict[str, Any], cwd: object) -> bool:
    if not isinstance(cwd, str) or not Path(cwd).is_absolute():
        return False
    operation = tool.rsplit("github_", 1)[-1]
    if operation in READ_OPERATIONS:
        return True
    if repository_policy_status(cwd) is None:
        return False
    fields = FIELDS.get(operation)
    if fields is None or set(data) - fields or not _owned(data, cwd):
        return False
    mutation = _mutation(operation, data)
    return mutation is not None and admitted(mutation)


def _owned(data: dict[str, Any], cwd: str) -> bool:
    repository = data.get("repository_full_name", data.get("repo_full_name"))
    if not isinstance(repository, str):
        return False
    selected = _repository_identity(repository)
    owned = repository_identity(cwd)
    return selected is not None and owned is not None and selected == owned


def _repository_identity(value: str) -> tuple[str, str, str] | None:
    owner, separator, repository = value.partition("/")
    if separator != "/" or "/" in repository or not _owner(owner) or not _repository(repository):
        return None
    return "github.com", owner.casefold(), repository.casefold()


def _owner(value: str) -> bool:
    return bool(value) and value[0].isascii() and value[0].isalnum() and all(
        character.isascii() and (character.isalnum() or character == "-") for character in value
    )


def _repository(value: str) -> bool:
    return bool(value) and all(
        character.isascii() and (character.isalnum() or character in "._-") for character in value
    )


def _mutation(operation: str, data: dict[str, Any]) -> Mutation | None:
    number = data.get("issue_number", data.get("pr_number"))
    if number is not None and (type(number) is not int or number < 1):
        return None
    if operation == "create_issue":
        return Mutation(MutationKind.ISSUE_CREATE, True, payload=_payload(data, {"repository_full_name"}))
    if operation == "update_issue":
        return Mutation(MutationKind.ISSUE_UPDATE, True, number, payload=_payload(data, {"repository_full_name", "issue_number"}))
    if operation == "create_pull_request":
        return Mutation(MutationKind.PR_CREATE, True, payload=_payload(data, {"repository_full_name"}, {"base_branch": "base", "head_branch": "head"}))
    if operation == "update_pull_request":
        return Mutation(MutationKind.PR_UPDATE, True, number, payload=_payload(data, {"repository_full_name", "pr_number"}, {"base_branch": "base"}))
    if operation == "add_comment_to_issue":
        return Mutation(MutationKind.PR_UPDATE, True, number, operation="pull_request.comment", payload={"comment": data.get("comment")})
    if operation in {"add_issue_labels", "remove_issue_label"}:
        labels = data.get("labels", [data.get("label")])
        return Mutation(MutationKind.ISSUE_UPDATE, True, number, operation="issue.set_labels", payload={"labels": labels})
    if operation in {"add_issue_assignees", "remove_issue_assignees"}:
        return Mutation(MutationKind.ISSUE_UPDATE, True, number, operation="issue.set_assignees", payload={"assignees": data.get("assignees")})
    if operation == "add_review_to_pr":
        payload = {"action": data.get("action")}
        if "review" in data: payload["body"] = data["review"]
        if "commit_id" in data: payload["commit_id"] = data["commit_id"]
        if "file_comments" in data: payload["file_comments"] = data["file_comments"]
        return Mutation(MutationKind.PR_UPDATE, True, number, operation="pull_request.submit_review", payload=payload)
    if operation in {"request_pull_request_reviewers", "remove_pull_request_reviewers"}:
        return Mutation(MutationKind.PR_UPDATE, True, number, operation="pull_request.set_reviewers", payload=_payload(data, {"repository_full_name", "pr_number"}))
    if operation == "convert_pull_request_to_draft":
        return Mutation(MutationKind.PR_UPDATE, True, number, operation="pull_request.convert_to_draft", payload={"transition": "draft"})
    if operation == "mark_pull_request_ready_for_review":
        return Mutation(MutationKind.PR_UPDATE, True, number, operation="pull_request.mark_ready", payload={"transition": "ready"})
    if operation in {"merge_pull_request", "enable_auto_merge"}:
        return Mutation(MutationKind.PR_MERGE, True, number)
    return None


def _payload(data: dict[str, Any], excluded: set[str], aliases: dict[str, str] | None = None) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in data.items():
        if key in excluded:
            continue
        name = (aliases or {}).get(key, key)
        if name in result:
            return {}
        result[name] = value
    return result
