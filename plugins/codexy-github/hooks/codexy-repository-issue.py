#!/usr/bin/python3
"""Repository GitHub-connector hook entrypoint."""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))

from codexy_policy.envelope import evaluate
from codexy_policy.repository_issue import forbidden

TOOLS = frozenset(
    {
        "mcp__codex_apps__github_add_comment_to_issue",
        "mcp__codex_apps__github_add_issue_assignees",
        "mcp__codex_apps__github_add_issue_labels",
        "mcp__codex_apps__github_add_reaction_to_issue_comment",
        "mcp__codex_apps__github_add_reaction_to_pr",
        "mcp__codex_apps__github_add_reaction_to_pr_review_comment",
        "mcp__codex_apps__github_add_review_to_pr",
        "mcp__codex_apps__github_compare_commits",
        "mcp__codex_apps__github_convert_pull_request_to_draft",
        "mcp__codex_apps__github_create_blob",
        "mcp__codex_apps__github_create_branch",
        "mcp__codex_apps__github_create_commit",
        "mcp__codex_apps__github_create_file",
        "mcp__codex_apps__github_create_issue",
        "mcp__codex_apps__github_create_pull_request",
        "mcp__codex_apps__github_create_tree",
        "mcp__codex_apps__github_delete_file",
        "mcp__codex_apps__github_dismiss_pull_request_review",
        "mcp__codex_apps__github_download_user_content",
        "mcp__codex_apps__github_download_workflow_artifact",
        "mcp__codex_apps__github_enable_auto_merge",
        "mcp__codex_apps__github_fetch",
        "mcp__codex_apps__github_fetch_blob",
        "mcp__codex_apps__github_fetch_commit",
        "mcp__codex_apps__github_fetch_commit_workflow_runs",
        "mcp__codex_apps__github_fetch_file",
        "mcp__codex_apps__github_fetch_issue",
        "mcp__codex_apps__github_fetch_issue_comments",
        "mcp__codex_apps__github_fetch_pr",
        "mcp__codex_apps__github_fetch_pr_comments",
        "mcp__codex_apps__github_fetch_pr_file_patch",
        "mcp__codex_apps__github_fetch_pr_patch",
        "mcp__codex_apps__github_fetch_workflow_job_logs",
        "mcp__codex_apps__github_fetch_workflow_job_steps",
        "mcp__codex_apps__github_fetch_workflow_run_artifacts",
        "mcp__codex_apps__github_fetch_workflow_run_jobs",
        "mcp__codex_apps__github_get_commit_combined_status",
        "mcp__codex_apps__github_get_issue_comment_reactions",
        "mcp__codex_apps__github_get_pr_diff",
        "mcp__codex_apps__github_get_pr_info",
        "mcp__codex_apps__github_get_pr_reactions",
        "mcp__codex_apps__github_get_pr_review_comment_reactions",
        "mcp__codex_apps__github_get_profile",
        "mcp__codex_apps__github_get_repo",
        "mcp__codex_apps__github_get_repo_collaborator_permission",
        "mcp__codex_apps__github_get_user_login",
        "mcp__codex_apps__github_get_users_recent_prs_in_repo",
        "mcp__codex_apps__github_label_pr",
        "mcp__codex_apps__github_list_installations",
        "mcp__codex_apps__github_list_installed_accounts",
        "mcp__codex_apps__github_list_pr_changed_filenames",
        "mcp__codex_apps__github_list_pull_request_review_threads",
        "mcp__codex_apps__github_list_pull_request_reviews",
        "mcp__codex_apps__github_list_recent_issues",
        "mcp__codex_apps__github_list_repositories",
        "mcp__codex_apps__github_list_repositories_by_affiliation",
        "mcp__codex_apps__github_list_repositories_by_installation",
        "mcp__codex_apps__github_list_user_org_memberships",
        "mcp__codex_apps__github_list_user_orgs",
        "mcp__codex_apps__github_lock_issue_conversation",
        "mcp__codex_apps__github_mark_pull_request_ready_for_review",
        "mcp__codex_apps__github_merge_pull_request",
        "mcp__codex_apps__github_remove_issue_assignees",
        "mcp__codex_apps__github_remove_issue_label",
        "mcp__codex_apps__github_remove_pull_request_reviewers",
        "mcp__codex_apps__github_remove_reaction_from_issue_comment",
        "mcp__codex_apps__github_remove_reaction_from_pr",
        "mcp__codex_apps__github_remove_reaction_from_pr_review_comment",
        "mcp__codex_apps__github_reply_to_review_comment",
        "mcp__codex_apps__github_request_pull_request_reviewers",
        "mcp__codex_apps__github_rerun_failed_workflow_run_jobs",
        "mcp__codex_apps__github_rerun_workflow_job",
        "mcp__codex_apps__github_resolve_review_thread",
        "mcp__codex_apps__github_search",
        "mcp__codex_apps__github_search_branches",
        "mcp__codex_apps__github_search_commits",
        "mcp__codex_apps__github_search_installed_repositories_streaming",
        "mcp__codex_apps__github_search_installed_repositories_v2",
        "mcp__codex_apps__github_search_issues",
        "mcp__codex_apps__github_search_prs",
        "mcp__codex_apps__github_search_repositories",
        "mcp__codex_apps__github_unlock_issue_conversation",
        "mcp__codex_apps__github_unresolve_review_thread",
        "mcp__codex_apps__github_update_file",
        "mcp__codex_apps__github_update_issue",
        "mcp__codex_apps__github_update_issue_comment",
        "mcp__codex_apps__github_update_pull_request",
        "mcp__codex_apps__github_update_ref",
        "mcp__codex_apps__github_update_review_comment",
    }
)


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument(
        "--event", required=True, choices=("PreToolUse", "PermissionRequest")
    )
    event = parser.parse_args().event
    output = evaluate(
        event,
        sys.stdin.buffer.read(1024 * 1024 + 1),
        TOOLS,
        "CODEXY_REPOSITORY_ISSUE_",
        forbidden,
    )
    if output:
        sys.stdout.buffer.write(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
