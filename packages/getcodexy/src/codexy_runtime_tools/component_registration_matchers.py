"""Readable literal matcher definitions for component registrations."""

from __future__ import annotations

import re


GITHUB_CONNECTOR_PREFIX = "mcp__codex_apps__github_"
GITHUB_ISSUE_TOOL_NAMES = (
    "add_comment_to_issue",
    "add_issue_assignees",
    "add_issue_labels",
    "add_reaction_to_issue_comment",
    "add_reaction_to_pr",
    "add_reaction_to_pr_review_comment",
    "add_review_to_pr",
    "compare_commits",
    "convert_pull_request_to_draft",
    "create_blob",
    "create_branch",
    "create_commit",
    "create_file",
    "create_issue",
    "create_tree",
    "delete_file",
    "dismiss_pull_request_review",
    "download_user_content",
    "download_workflow_artifact",
    "fetch",
    "fetch_blob",
    "fetch_commit",
    "fetch_commit_workflow_runs",
    "fetch_file",
    "fetch_issue",
    "fetch_issue_comments",
    "fetch_pr",
    "fetch_pr_comments",
    "fetch_pr_file_patch",
    "fetch_pr_patch",
    "fetch_workflow_job_logs",
    "fetch_workflow_job_steps",
    "fetch_workflow_run_artifacts",
    "fetch_workflow_run_jobs",
    "get_commit_combined_status",
    "get_issue_comment_reactions",
    "get_pr_diff",
    "get_pr_info",
    "get_pr_reactions",
    "get_pr_review_comment_reactions",
    "get_profile",
    "get_repo",
    "get_repo_collaborator_permission",
    "get_user_login",
    "get_users_recent_prs_in_repo",
    "label_pr",
    "list_installations",
    "list_installed_accounts",
    "list_pr_changed_filenames",
    "list_pull_request_review_threads",
    "list_pull_request_reviews",
    "list_recent_issues",
    "list_repositories",
    "list_repositories_by_affiliation",
    "list_repositories_by_installation",
    "list_user_org_memberships",
    "list_user_orgs",
    "lock_issue_conversation",
    "mark_pull_request_ready_for_review",
    "remove_issue_assignees",
    "remove_issue_label",
    "remove_pull_request_reviewers",
    "remove_reaction_from_issue_comment",
    "remove_reaction_from_pr",
    "remove_reaction_from_pr_review_comment",
    "reply_to_review_comment",
    "request_pull_request_reviewers",
    "rerun_failed_workflow_run_jobs",
    "rerun_workflow_job",
    "resolve_review_thread",
    "search",
    "search_branches",
    "search_commits",
    "search_installed_repositories_streaming",
    "search_installed_repositories_v2",
    "search_issues",
    "search_prs",
    "search_repositories",
    "unlock_issue_conversation",
    "unresolve_review_thread",
    "update_file",
    "update_issue",
    "update_issue_comment",
    "update_ref",
    "update_review_comment",
)


def _escaped_alternation(values: tuple[str, ...]) -> str:
    return "|".join(re.escape(value) for value in values)


def build_literal_matcher(prefix: str, values: tuple[str, ...]) -> str:
    """Build an anchored matcher for literal names under a namespace prefix."""
    if not values or any(not value for value in values):
        raise ValueError("a matcher requires non-empty literal values")
    escaped_prefix = re.escape(prefix)
    escaped_values = _escaped_alternation(values)
    if len(values) == 1:
        return f"^{escaped_prefix}{escaped_values}$"
    return f"^{escaped_prefix}(?:{escaped_values})$"


ISSUE_MATCHER = build_literal_matcher(GITHUB_CONNECTOR_PREFIX, GITHUB_ISSUE_TOOL_NAMES)
PULL_REQUEST_MATCHER = r"^mcp__codex_apps__github_(create|update)_pull_request$"
MERGE_MATCHER = r"^mcp__codex_apps__github_(merge_pull_request|enable_auto_merge)$"
FUNCTIONS_EXEC_MATCHER = build_literal_matcher("", ("functions.exec",))
