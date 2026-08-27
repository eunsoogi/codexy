"""Canonical ordinary-file registration checks used by the doctor report."""

from __future__ import annotations

import json
import os
from pathlib import Path

from .component_integrity import verify_component


CATALOGS = {
    "core": """# Codexy packaged-agent discovery/registration contract. Validators and the
# registration script load agent_files from this catalog; native Codex agent
# use is through marker-owned standalone files under the Codex home agents directory.
version = "0.1.0"
catalog_kind = "plugin-packaged-specialist-agent-files"
native_custom_agent_registration = "codex-home-standalone-agent-projection"
native_custom_agent_projection = "managed-codexy-subdirectory"
agent_files = [
  "codexy-architect.toml",
  "codexy-cartographer.toml",
  "codexy-auditor.toml",
  "codexy-shipwright.toml",
  "codexy-inspector.toml",
  "codexy-sentinel.toml",
  "codexy-warden.toml",
]
""",
    "github": """version = "0.1.0"
catalog_kind = "plugin-packaged-specialist-agent-files"
agent_files = ["codexy-weaver.toml"]
""",
}
AGENT_FILES = {
    "core": (
        "codexy-architect.toml",
        "codexy-cartographer.toml",
        "codexy-auditor.toml",
        "codexy-shipwright.toml",
        "codexy-inspector.toml",
        "codexy-sentinel.toml",
        "codexy-warden.toml",
    ),
    "github": ("codexy-weaver.toml",),
}


def _command_hook(matcher: str, launcher_stem: str, event: str) -> dict[str, object]:
    return {
        "matcher": matcher,
        "hooks": [
            {
                "type": "command",
                "command": f'"${{PLUGIN_ROOT}}/hooks/{launcher_stem}.sh" {event}',
                "commandWindows": f'"${{PLUGIN_ROOT}}/hooks/{launcher_stem}.cmd" {event}',
                "timeout": 5,
            }
        ],
    }


def _native_hook(matcher: str, rule: str) -> dict[str, object]:
    return {
        "matcher": matcher,
        "hooks": [
            {
                "type": "command",
                "command": f'"${{PLUGIN_ROOT}}/hooks/codexy-github-admission.sh" --rule {rule}',
                "commandWindows": f'"${{PLUGIN_ROOT}}/hooks/codexy-github-admission-{rule}.cmd"',
                "timeout": 5,
            }
        ],
    }


ISSUE_MATCHER = r"^mcp__codex_apps__github_(?:add_comment_to_issue|add_issue_assignees|add_issue_labels|add_reaction_to_issue_comment|add_reaction_to_pr|add_reaction_to_pr_review_comment|add_review_to_pr|compare_commits|convert_pull_request_to_draft|create_blob|create_branch|create_commit|create_file|create_issue|create_tree|delete_file|dismiss_pull_request_review|download_user_content|download_workflow_artifact|fetch|fetch_blob|fetch_commit|fetch_commit_workflow_runs|fetch_file|fetch_issue|fetch_issue_comments|fetch_pr|fetch_pr_comments|fetch_pr_file_patch|fetch_pr_patch|fetch_workflow_job_logs|fetch_workflow_job_steps|fetch_workflow_run_artifacts|fetch_workflow_run_jobs|get_commit_combined_status|get_issue_comment_reactions|get_pr_diff|get_pr_info|get_pr_reactions|get_pr_review_comment_reactions|get_profile|get_repo|get_repo_collaborator_permission|get_user_login|get_users_recent_prs_in_repo|label_pr|list_installations|list_installed_accounts|list_pr_changed_filenames|list_pull_request_review_threads|list_pull_request_reviews|list_recent_issues|list_repositories|list_repositories_by_affiliation|list_repositories_by_installation|list_user_org_memberships|list_user_orgs|lock_issue_conversation|mark_pull_request_ready_for_review|remove_issue_assignees|remove_issue_label|remove_pull_request_reviewers|remove_reaction_from_issue_comment|remove_reaction_from_pr|remove_reaction_from_pr_review_comment|reply_to_review_comment|request_pull_request_reviewers|rerun_failed_workflow_run_jobs|rerun_workflow_job|resolve_review_thread|search|search_branches|search_commits|search_installed_repositories_streaming|search_installed_repositories_v2|search_issues|search_prs|search_repositories|unlock_issue_conversation|unresolve_review_thread|update_file|update_issue|update_issue_comment|update_ref|update_review_comment)$"
CONNECTOR_HOOKS = (
    (ISSUE_MATCHER, "codexy-repository-issue"),
    (
        r"^mcp__codex_apps__github_(create|update)_pull_request$",
        "codexy-repository-pull-request",
    ),
    (
        r"^mcp__codex_apps__github_(merge_pull_request|enable_auto_merge)$",
        "codexy-repository-merge",
    ),
)


def _connector_hooks(event: str) -> list[dict[str, object]]:
    return [
        _command_hook(matcher, launcher, event) for matcher, launcher in CONNECTOR_HOOKS
    ]


def _bash_hooks(event: str) -> list[dict[str, object]]:
    return [
        _command_hook("^Bash$", stem, event)
        for stem in ("codexy-repository-github-command", "codexy-destructive-command")
    ]


_CORE_COMMAND_HOOKS = (
    ("^codex_app__send_message_to_thread$", "codexy-thread-delivery"),
    ("^codex_app__create_thread$", "codexy-child-thread-creation"),
)


HOOKS = {
    "core": {
        "hooks": {
            event: [
                _command_hook(matcher, launcher_stem, event)
                for matcher, launcher_stem in _CORE_COMMAND_HOOKS
            ]
            for event in ("PermissionRequest", "PreToolUse")
        }
    },
    "github": {
        "hooks": {
            "UserPromptSubmit": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": '"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.sh"',
                            "commandWindows": '"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.cmd"',
                            "timeout": 5,
                        }
                    ]
                }
            ],
            "PermissionRequest": [
                *_connector_hooks("PermissionRequest"),
                *_bash_hooks("PermissionRequest"),
            ],
            "PreToolUse": [
                _native_hook("^mcp__codex_apps__github_create_issue$", "issue"),
                _native_hook("^mcp__codex_apps__github_create_pull_request$", "pr"),
                *_connector_hooks("PreToolUse"),
                *_bash_hooks("PreToolUse"),
            ],
        }
    },
}
MCP = {
    "lsp": {
        "command": "./mcp/codexy-mcp-devtools",
        "args": ["lsp", "--stdio"],
        "cwd": ".",
    },
    "codegraph": {
        "command": "./mcp/codexy-mcp-devtools",
        "args": ["codegraph", "--stdio"],
        "cwd": ".",
    },
}
LAUNCHERS = {
    "core": (
        "hooks/codexy-thread-delivery.sh",
        "hooks/codexy-thread-delivery.cmd",
        "hooks/codexy-child-thread-creation.sh",
        "hooks/codexy-child-thread-creation.cmd",
    ),
    "github": (
        "hooks/codexy-github-workflow-context.sh",
        "hooks/codexy-github-workflow-context.cmd",
        "hooks/codexy-github-admission.sh",
        "hooks/codexy-github-admission-issue.cmd",
        "hooks/codexy-github-admission-pr.cmd",
        "hooks/codexy-repository-issue.sh",
        "hooks/codexy-repository-issue.cmd",
        "hooks/codexy-repository-pull-request.sh",
        "hooks/codexy-repository-pull-request.cmd",
        "hooks/codexy-repository-merge.sh",
        "hooks/codexy-repository-merge.cmd",
        "hooks/codexy-repository-github-command.sh",
        "hooks/codexy-repository-github-command.cmd",
        "hooks/codexy-destructive-command.sh",
        "hooks/codexy-destructive-command.cmd",
    ),
    "devtools": ("mcp/codexy-mcp-devtools",),
}
CORE_HOOK_DEPENDENCIES = (
    "hooks/codexy-child-thread-creation.py",
    "hooks/codexy_policy/child_thread_creation.py",
    "hooks/codexy_policy/envelope.py",
)


def valid_registration(plugin: Path, component: str) -> bool:
    """Require exactly the packaged registration and its local launch targets."""
    try:
        if component == "devtools":
            return _json(plugin / ".mcp.json") == MCP and _executable(
                plugin / LAUNCHERS[component][0]
            )
        verify_component(plugin, "codexy" if component == "core" else "codexy-github")
        return (
            _text(plugin / "agents/catalog.toml") == CATALOGS[component]
            and _json(plugin / "hooks/hooks.json") == HOOKS[component]
            and all(
                _regular(plugin / f"agents/{name}") for name in AGENT_FILES[component]
            )
            and all(_regular(plugin / path) for path in LAUNCHERS[component])
            and (
                component != "core"
                or all(_regular(plugin / path) for path in CORE_HOOK_DEPENDENCIES)
            )
        )
    except (KeyError, OSError, UnicodeDecodeError, ValueError):
        return False


def _json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as source:
        return json.load(source)


def _text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _regular(path: Path) -> bool:
    try:
        return path.is_file() and not path.is_symlink() and path.stat().st_size > 0
    except OSError:
        return False


def _executable(path: Path) -> bool:
    return _regular(path) and os.access(path, os.X_OK)
