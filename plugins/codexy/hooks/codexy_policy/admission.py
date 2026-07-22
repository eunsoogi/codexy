"""Official-event policy evaluation with deterministic deny-or-zero output."""

from __future__ import annotations

import json
import os
from typing import Any

from .github import connector_admitted
from .merge import valid as merge_valid
from .shell import forbidden as shell_forbidden

MAX_INPUT = 1024 * 1024
OWNED = "eunsoogi/codexy"
THREAD = "codex_app__send_message_to_thread"
FIELDS = {
    "mcp__codex_apps__github_create_issue": {"assignees", "body", "labels", "milestone", "repository_full_name", "title"},
    "mcp__codex_apps__github_update_issue": {"assignees", "body", "issue_number", "labels", "milestone", "repository_full_name", "state", "state_reason", "title"},
    "mcp__codex_apps__github_create_pull_request": {"base", "base_branch", "body", "draft", "head", "head_branch", "head_repo", "issue", "maintainer_can_modify", "repository_full_name", "title"},
    "mcp__codex_apps__github_update_pull_request": {"base_branch", "body", "maintainer_can_modify", "pr_number", "repository_full_name", "state", "title"},
    "mcp__codex_apps__github_merge_pull_request": {"commit_message", "commit_title", "expected_head_sha", "merge_method", "pr_number", "repository_full_name"},
    "mcp__codex_apps__github_enable_auto_merge": {"pr_number", "repository_full_name"},
}


def _pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in items:
        if key in result:
            raise ValueError("duplicate key")
        result[key] = value
    return result


def deny(event: str) -> bytes:
    reason = "Codexy policy: MUST NOT execute an invalid or forbidden owned operation."
    if event == "PermissionRequest":
        output = {"hookEventName": event, "decision": {"behavior": "deny", "message": reason}}
    else:
        output = {"hookEventName": event, "permissionDecision": "deny", "permissionDecisionReason": reason}
    return (json.dumps({"hookSpecificOutput": output}, separators=(",", ":")) + "\n").encode()


def evaluate(event: str, payload: bytes) -> bytes:
    if event not in {"PreToolUse", "PermissionRequest"} or len(payload) > MAX_INPUT:
        return deny(event if event in {"PreToolUse", "PermissionRequest"} else "PreToolUse")
    try:
        data = json.loads(payload.decode("utf-8", "strict"), object_pairs_hook=_pairs)
    except (UnicodeError, ValueError, json.JSONDecodeError):
        return deny(event)
    if not isinstance(data, dict) or data.get("hook_event_name") != event or not isinstance(data.get("tool_name"), str):
        return deny(event)
    tool, tool_input = data["tool_name"], data.get("tool_input")
    if tool == THREAD:
        return b"" if isinstance(tool_input, dict) and _nonblank(tool_input, "model") and _nonblank(tool_input, "thinking") else deny(event)
    if tool in FIELDS:
        return _github(event, tool, tool_input)
    if tool != "Bash":
        return b""
    if not isinstance(tool_input, dict) or not isinstance(tool_input.get("command"), str) or not isinstance(data.get("cwd"), str):
        return deny(event)
    inherited_repository = os.environ.get("GH_REPO") or None
    inherited_git_dir = os.environ.get("GIT_DIR") or None
    inherited_git_config = tuple(
        (key, value) for key, value in os.environ.items() if key.startswith("GIT_CONFIG_")
    )
    return deny(event) if shell_forbidden(tool_input["command"], data["cwd"], inherited_repository, inherited_git_dir, inherited_git_config) else b""


def _github(event: str, tool: str, data: object) -> bytes:
    if not isinstance(data, dict) or not set(data).issubset(FIELDS[tool]):
        return deny(event)
    repository = data.get("repository_full_name")
    if not isinstance(repository, str) or repository.lower() == OWNED and repository != repository.strip():
        return deny(event)
    if repository.lower() != OWNED:
        return b""
    invalid = False
    if tool.endswith("enable_auto_merge"):
        invalid = True
    elif tool.endswith("merge_pull_request"):
        invalid = not merge_valid(data)
    else:
        invalid = not connector_admitted(tool, data)
    return deny(event) if invalid else b""


def _nonblank(data: dict[str, Any], field: str) -> bool:
    return isinstance(data.get(field), str) and bool(data[field].strip())
