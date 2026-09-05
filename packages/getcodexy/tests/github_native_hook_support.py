"""Host-command helpers shared by GitHub native hook integration cases."""

import json
import os
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins" / "codexy-github"

ISSUE_MATCHER = r"^mcp__codex_apps__github_(?:add_comment_to_issue|add_issue_assignees|add_issue_labels|add_reaction_to_issue_comment|add_reaction_to_pr|add_reaction_to_pr_review_comment|add_review_to_pr|compare_commits|convert_pull_request_to_draft|create_blob|create_branch|create_commit|create_file|create_issue|create_tree|delete_file|dismiss_pull_request_review|download_user_content|download_workflow_artifact|fetch|fetch_blob|fetch_commit|fetch_commit_workflow_runs|fetch_file|fetch_issue|fetch_issue_comments|fetch_pr|fetch_pr_comments|fetch_pr_file_patch|fetch_pr_patch|fetch_workflow_job_logs|fetch_workflow_job_steps|fetch_workflow_run_artifacts|fetch_workflow_run_jobs|get_commit_combined_status|get_issue_comment_reactions|get_pr_diff|get_pr_info|get_pr_reactions|get_pr_review_comment_reactions|get_profile|get_repo|get_repo_collaborator_permission|get_user_login|get_users_recent_prs_in_repo|label_pr|list_installations|list_installed_accounts|list_pr_changed_filenames|list_pull_request_review_threads|list_pull_request_reviews|list_recent_issues|list_repositories|list_repositories_by_affiliation|list_repositories_by_installation|list_user_org_memberships|list_user_orgs|lock_issue_conversation|mark_pull_request_ready_for_review|remove_issue_assignees|remove_issue_label|remove_pull_request_reviewers|remove_reaction_from_issue_comment|remove_reaction_from_pr|remove_reaction_from_pr_review_comment|reply_to_review_comment|request_pull_request_reviewers|rerun_failed_workflow_run_jobs|rerun_workflow_job|resolve_review_thread|search|search_branches|search_commits|search_installed_repositories_streaming|search_installed_repositories_v2|search_issues|search_prs|search_repositories|unlock_issue_conversation|unresolve_review_thread|update_file|update_issue|update_issue_comment|update_ref|update_review_comment)$"
PR_MATCHER = r"^mcp__codex_apps__github_(create|update)_pull_request$"
MERGE_MATCHER = r"^mcp__codex_apps__github_(merge_pull_request|enable_auto_merge)$"
NESTED_EXEC_MATCHER = r"^functions\.exec$"


class GithubNativeHookSupport:
    @staticmethod
    def expected_pre_tool_use_admissions() -> tuple[
        tuple[str, tuple[tuple[str, str, str, int], ...]], ...
    ]:
        return (
            (
                "^mcp__codex_apps__github_create_issue$",
                (
                    (
                        "command",
                        '"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh" --rule issue',
                        '"${PLUGIN_ROOT}/hooks/codexy-github-admission-issue.cmd"',
                        5,
                    ),
                ),
            ),
            (
                "^mcp__codex_apps__github_create_pull_request$",
                (
                    (
                        "command",
                        '"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh" --rule pr',
                        '"${PLUGIN_ROOT}/hooks/codexy-github-admission-pr.cmd"',
                        5,
                    ),
                ),
            ),
            (
                ISSUE_MATCHER,
                (
                    (
                        "command",
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-issue.sh" PreToolUse',
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-issue.cmd" PreToolUse',
                        5,
                    ),
                ),
            ),
            (
                PR_MATCHER,
                (
                    (
                        "command",
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-pull-request.sh" PreToolUse',
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-pull-request.cmd" PreToolUse',
                        5,
                    ),
                ),
            ),
            (
                MERGE_MATCHER,
                (
                    (
                        "command",
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-merge.sh" PreToolUse',
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-merge.cmd" PreToolUse',
                        5,
                    ),
                ),
            ),
            (
                NESTED_EXEC_MATCHER,
                (
                    (
                        "command",
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-github-exec.sh" PreToolUse',
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-github-exec.cmd" PreToolUse',
                        5,
                    ),
                ),
            ),
            (
                "^Bash$",
                (
                    (
                        "command",
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-github-command.sh" PreToolUse',
                        '"${PLUGIN_ROOT}/hooks/codexy-repository-github-command.cmd" PreToolUse',
                        5,
                    ),
                ),
            ),
            (
                "^Bash$",
                (
                    (
                        "command",
                        '"${PLUGIN_ROOT}/hooks/codexy-destructive-command.sh" PreToolUse',
                        '"${PLUGIN_ROOT}/hooks/codexy-destructive-command.cmd" PreToolUse',
                        5,
                    ),
                ),
            ),
        )

    @staticmethod
    def _admission_contract(
        admissions: list[dict[str, object]],
    ) -> tuple[tuple[str, tuple[tuple[str, str, str, int], ...]], ...]:
        return tuple(
            (
                admission["matcher"],
                tuple(
                    (
                        hook["type"],
                        hook["command"],
                        hook["commandWindows"],
                        hook["timeout"],
                    )
                    for hook in admission["hooks"]
                ),
            )
            for admission in admissions
        )

    @staticmethod
    def _admission(
        installed: Path,
        environment: dict[str, str],
        rule: str,
        title: str,
        denied: bool,
    ) -> None:
        GithubNativeHookSupport._admission_payload(
            installed,
            environment,
            rule,
            {
                "tool_name": "mcp__codex_apps__github_create_issue",
                "tool_input": {"title": title},
            },
            denied,
        )

    @staticmethod
    def _admission_payload(
        installed: Path,
        environment: dict[str, str],
        rule: str,
        payload: dict[str, object],
        denied: bool,
    ) -> None:
        GithubNativeHookSupport._admission_raw(
            installed, environment, rule, json.dumps(payload), denied
        )

    @staticmethod
    def _admission_raw(
        installed: Path,
        environment: dict[str, str],
        rule: str,
        payload: str,
        denied: bool,
    ) -> None:
        if os.name == "nt":
            command = [str(installed / f"hooks/codexy-github-admission-{rule}.cmd")]
        else:
            command = [
                str(installed / "hooks/codexy-github-admission.sh"),
                "--rule",
                rule,
            ]
        result = subprocess.run(
            command,
            input=payload,
            text=True,
            capture_output=True,
            env={**environment, "PLUGIN_ROOT": str(installed)},
            check=False,
        )
        if result.returncode:
            raise AssertionError(result.stderr)
        assert ("permissionDecision" in result.stdout) == denied, result.stdout

    @staticmethod
    def _run(path: Path, *arguments: str) -> None:
        result = subprocess.run(
            [str(path), *arguments], text=True, capture_output=True, check=False
        )
        if result.returncode:
            raise AssertionError(f"{path.name} failed:\n{result.stdout}{result.stderr}")

    @staticmethod
    def _host(environment: dict[str, str], *arguments: str) -> dict[str, object]:
        result = subprocess.run(
            ["codex", *arguments, "--json"],
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode:
            raise AssertionError(
                f"codex {' '.join(arguments)} failed:\n{result.stdout}{result.stderr}"
            )
        return json.loads(result.stdout)

    @staticmethod
    def _assert_enabled_plugins(
        inventory: dict[str, object], expected: set[str]
    ) -> None:
        installed = inventory.get("installed")
        if not isinstance(installed, list):
            raise AssertionError(f"missing installed plugin inventory: {inventory}")
        enabled = {
            entry.get("pluginId")
            for entry in installed
            if isinstance(entry, dict) and entry.get("enabled") is True
        }
        assert enabled == expected, enabled
