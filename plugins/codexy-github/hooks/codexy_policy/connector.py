from pathlib import Path
import re
from typing import Any

from .connector_operation import operation
from .merge import positive_int
from .repository import github_identity, repository_identity, repository_policy_status
from .titles import issue_title, pr_title

READ_OPERATIONS = frozenset(
    """compare_commits download_user_content download_workflow_artifact fetch fetch_blob fetch_commit fetch_commit_workflow_runs fetch_file fetch_issue fetch_issue_comments fetch_pr fetch_pr_comments fetch_pr_file_patch fetch_pr_patch fetch_workflow_job_logs fetch_workflow_job_steps fetch_workflow_run_artifacts fetch_workflow_run_jobs get_commit_combined_status get_issue_comment_reactions get_pr_diff get_pr_info get_pr_reactions get_pr_review_comment_reactions get_profile get_repo get_repo_collaborator_permission get_user_login get_users_recent_prs_in_repo list_installations list_installed_accounts list_pr_changed_filenames list_pull_request_review_threads list_pull_request_reviews list_recent_issues list_repositories list_repositories_by_affiliation list_repositories_by_installation list_user_org_memberships list_user_orgs search search_branches search_commits search_installed_repositories_streaming search_installed_repositories_v2 search_issues search_prs search_repositories""".split()
)


def connector_admitted(tool: str, data: dict[str, Any], cwd: object) -> bool:
    if not isinstance(cwd, str) or not Path(cwd).is_absolute():
        return False
    return operation(tool) in READ_OPERATIONS or (
        repository_policy_status(cwd) is not None
        and _owned(data, cwd)
        and _eligible(operation(tool), data)
    )


def _owned(data: dict[str, Any], cwd: str) -> bool:
    repository = data.get("repository_full_name", data.get("repo_full_name"))
    if not isinstance(repository, str):
        return False
    valid = re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]*/[A-Za-z0-9._-]+", repository)
    selected = github_identity(repository) if valid else None
    return selected is not None and repository_identity(cwd) == selected


def _eligible(operation: str, data: dict[str, Any]) -> bool:
    number = data.get("issue_number", data.get("pr_number"))
    if (number is not None and (type(number) is not int or number < 1)) or (
        operation not in {"create_issue", "create_pull_request"} and number is None
    ):
        return False
    if operation == "create_issue":
        return _issue_create(_payload(data, {"repository_full_name"}))
    if operation == "update_issue":
        return _issue_update(_payload(data, {"repository_full_name", "issue_number"}))
    if operation == "create_pull_request":
        return _pr_create(
            _payload(
                data,
                {"repository_full_name"},
                {"base_branch": "base", "head_branch": "head"},
            )
        )
    if operation == "update_pull_request":
        return _pr_update(
            _payload(
                data, {"repository_full_name", "pr_number"}, {"base_branch": "base"}
            )
        )
    if operation == "add_comment_to_issue":
        return set(data) == {"comment", "pr_number", "repo_full_name"} and isinstance(
            data.get("comment"), str
        )
    if operation in set(
        "add_issue_labels add_issue_assignees remove_issue_assignees".split()
    ):
        field = "labels" if "labels" in operation else "assignees"
        return set(data) == set("repository_full_name issue_number".split()) | {
            field
        } and _strings(data.get(field))
    if operation == "remove_issue_label":
        return set(data) == set(
            "repository_full_name issue_number label".split()
        ) and _nonempty(data.get("label"))
    if operation == "add_review_to_pr":
        if set(data) - set(
            "action commit_id file_comments pr_number repo_full_name review".split()
        ):
            return False
        payload = {
            "body" if key == "review" else key: data[key]
            for key in ("action", "review", "commit_id", "file_comments")
            if key in data
        }
        return _review(payload)
    if operation in {"request_pull_request_reviewers", "remove_pull_request_reviewers"}:
        return _reviewers(_payload(data, {"repository_full_name", "pr_number"}))
    return set(data) == {"repository_full_name", "pr_number"} and operation in {
        "convert_pull_request_to_draft",
        "mark_pull_request_ready_for_review",
    }


def _issue_create(payload: dict[str, Any]) -> bool:
    return (
        set(payload) <= set("title body assignees labels milestone".split())
        and issue_title(payload.get("title"))
        and _optional(payload)
    )


def _issue_update(payload: dict[str, Any]) -> bool:
    keys = set(payload)
    if keys and keys <= {"title", "body"}:
        return _metadata(payload, issue_title)
    if "state" in keys:
        return _issue_state(payload)
    if keys in ({"labels"}, {"assignees"}):
        field = next(iter(keys))
        return _strings(payload[field], empty=True)
    return keys == {"milestone"} and _nonempty(payload["milestone"], milestone=True)


def _pr_create(payload: dict[str, Any]) -> bool:
    return (
        set(payload)
        <= set("title body base head draft maintainer_can_modify head_repo".split())
        and pr_title(payload.get("title"))
        and _nonempty(payload.get("base"))
        and _nonempty(payload.get("head"))
        and _optional(payload)
    )


def _pr_update(payload: dict[str, Any]) -> bool:
    if set(payload) == {"state"}:
        return payload["state"] in {"open", "closed"}
    if set(payload) <= {"title", "body", "base", "maintainer_can_modify"}:
        return bool(payload) and _metadata(payload, pr_title)
    return False


def _metadata(payload: dict[str, Any], title: Any) -> bool:
    title_value, body, base, editable = (
        payload.get(key) for key in ("title", "body", "base", "maintainer_can_modify")
    )
    return (
        (title_value is None or title(title_value))
        and (body is None or isinstance(body, str))
        and (base is None or _nonempty(base))
        and (editable is None or type(editable) is bool)
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
        or reason in {None, "completed", "not_planned"}
        and "duplicate_issue_id" not in payload
    )


def _review(payload: dict[str, Any]) -> bool:
    action = payload.get("action")
    return (
        set(payload) <= {"action", "body", "commit_id", "file_comments"}
        and action in {"COMMENT", "APPROVE", "REQUEST_CHANGES"}
        and (action == "APPROVE" or _nonempty(payload.get("body")))
        and (
            "body" not in payload
            or payload["body"] is None
            or isinstance(payload["body"], str)
        )
        and ("commit_id" not in payload or _nonempty(payload["commit_id"]))
        and (
            "file_comments" not in payload
            or payload["file_comments"] is None
            or _entries(payload["file_comments"])
        )
    )


def _entries(value: object) -> bool:
    fields = set("body path position line side start_line start_side".split())
    return isinstance(value, list) and all(
        isinstance(item, dict)
        and not set(item) - fields
        and _strings([item.get("body"), item.get("path")])
        and all(
            _review_value(key, item.get(key)) for key in set(item) - {"body", "path"}
        )
        for item in value
    )


def _review_value(key: str, value: object) -> bool:
    return value is None or (
        positive_int(value)
        if key in {"position", "line", "start_line"}
        else _nonempty(value)
    )


def _reviewers(payload: dict[str, Any]) -> bool:
    return (
        set(payload) <= {"reviewers", "team_reviewers"}
        and any(_strings(value) for value in payload.values())
        and all(value is None or _strings(value) for value in payload.values())
    )


def _optional(payload: dict[str, Any]) -> bool:
    return all(
        (key != "body" or isinstance(value, str) or value is None)
        and (
            key not in {"assignees", "labels"}
            or value is None
            or _strings(value, empty=True)
        )
        and (key != "milestone" or _nonempty(value, milestone=True))
        and (key != "draft" or type(value) is bool)
        and (key != "maintainer_can_modify" or value is None or type(value) is bool)
        and (
            key not in {"base", "head", "head_repo"}
            or value is None
            or _nonempty(value)
        )
        for key, value in payload.items()
    )


def _nonempty(value: object, *, milestone: bool = False) -> bool:
    return (isinstance(value, str) and bool(value.strip())) or (
        milestone and (value is None or positive_int(value))
    )


def _strings(value: object, *, empty: bool = False) -> bool:
    return (
        isinstance(value, list)
        and (empty or bool(value))
        and all(_nonempty(item) for item in value)
    )


def _payload(
    data: dict[str, Any], excluded: set[str], aliases: dict[str, str] | None = None
) -> dict[str, Any]:
    items = [
        ((aliases or {}).get(key, key), value)
        for key, value in data.items()
        if key not in excluded
    ]
    return {} if len(items) != len({key for key, _ in items}) else dict(items)
