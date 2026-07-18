from __future__ import annotations

import argparse
import json
import re
import sys
from typing import Any, Literal

from .shell_policy import repository_owned, shell_forbidden


Event = Literal["PreToolUse", "PermissionRequest"]
MAX_INPUT_BYTES = 1024 * 1024
CODEXY_REPOSITORY = "eunsoogi/codexy"
GITHUB_TOOLS = {
    "mcp__codex_apps__github_create_issue",
    "mcp__codex_apps__github_update_issue",
    "mcp__codex_apps__github_create_pull_request",
    "mcp__codex_apps__github_update_pull_request",
    "mcp__codex_apps__github_merge_pull_request",
    "mcp__codex_apps__github_enable_auto_merge",
}
THREAD_ROUTING_TOOL = "codex_app__send_message_to_thread"
GITHUB_FIELDS = {
    "mcp__codex_apps__github_create_issue": {
        "repository_full_name", "title", "body", "labels", "assignees", "milestone"
    },
    "mcp__codex_apps__github_update_issue": {
        "repository_full_name", "issue_number", "title", "body", "state", "state_reason",
        "labels", "assignees", "milestone",
    },
    "mcp__codex_apps__github_create_pull_request": {
        "repository_full_name", "title", "body", "head", "head_branch", "head_repo", "base",
        "base_branch", "draft", "maintainer_can_modify", "issue",
    },
    "mcp__codex_apps__github_update_pull_request": {
        "repository_full_name", "pr_number", "title", "body", "base_branch", "state",
        "maintainer_can_modify",
    },
    "mcp__codex_apps__github_merge_pull_request": {
        "repository_full_name", "pr_number", "expected_head_sha", "merge_method", "commit_title",
        "commit_message",
    },
    "mcp__codex_apps__github_enable_auto_merge": {
        "repository_full_name", "pr_number", "merge_method",
    },
}
CONVENTIONAL_TITLE = re.compile(
    r"^(?:feat|fix|docs|refactor|test|chore|ci|perf|build)"
    r"(?:\([a-z0-9._/-]+\))?!?: .+"
)
SHA = re.compile(r"^[0-9a-fA-F]{40}$")
FIXES = re.compile(r"(?:^|\n)Fixes #[1-9][0-9]*$")


class DuplicateKey(ValueError):
    pass


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKey(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _deny(event: Event) -> bytes:
    reason = "Codexy policy: MUST NOT execute an invalid or forbidden owned operation."
    if event == "PreToolUse":
        output = {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        }
    else:
        output = {
            "hookSpecificOutput": {
                "hookEventName": "PermissionRequest",
                "decision": {"behavior": "deny", "message": reason},
            }
        }
    return json.dumps(output, ensure_ascii=True, separators=(",", ":")).encode() + b"\n"


def _string(data: dict[str, Any], field: str) -> str | None:
    value = data.get(field)
    return value if isinstance(value, str) else None


def _valid_issue_title(title: str) -> bool:
    stripped = title.strip()
    return (
        len(stripped) >= 8
        and stripped[0].isupper()
        and not CONVENTIONAL_TITLE.fullmatch(stripped)
    )


def _nonblank_string(data: dict[str, Any], field: str) -> bool:
    value = data.get(field)
    return isinstance(value, str) and bool(value.strip())


def _github_forbidden(tool: str, tool_input: dict[str, Any]) -> bool:
    if not set(tool_input).issubset(GITHUB_FIELDS[tool]):
        return True
    if tool.endswith("enable_auto_merge"):
        return True
    if tool.endswith("create_issue"):
        title = _string(tool_input, "title")
        return title is None or not _valid_issue_title(title)
    if tool.endswith("update_issue"):
        if not isinstance(tool_input.get("issue_number"), int):
            return True
        title = tool_input.get("title")
        return title is not None and (not isinstance(title, str) or not _valid_issue_title(title))
    if tool.endswith("create_pull_request"):
        title = _string(tool_input, "title")
        return title is None or CONVENTIONAL_TITLE.fullmatch(title.strip()) is None
    if tool.endswith("update_pull_request"):
        if not isinstance(tool_input.get("pr_number"), int):
            return True
        title = tool_input.get("title")
        return title is not None and (
            not isinstance(title, str) or CONVENTIONAL_TITLE.fullmatch(title.strip()) is None
        )
    if tool.endswith("merge_pull_request"):
        pr_number = tool_input.get("pr_number")
        title = _string(tool_input, "commit_title")
        message = _string(tool_input, "commit_message")
        head = _string(tool_input, "expected_head_sha")
        return not (
            isinstance(pr_number, int)
            and pr_number > 0
            and tool_input.get("merge_method") == "squash"
            and head is not None
            and SHA.fullmatch(head)
            and title is not None
            and CONVENTIONAL_TITLE.fullmatch(title.strip())
            and title.rstrip().endswith(f"(#{pr_number})")
            and message is not None
            and FIXES.search(message.rstrip())
        )
    return False


def _evaluate_object(event: Event, payload: dict[str, Any]) -> bytes:
    if payload.get("hook_event_name") != event:
        return _deny(event)
    tool = payload.get("tool_name")
    if not isinstance(tool, str):
        return _deny(event)
    if tool == THREAD_ROUTING_TOOL:
        tool_input = payload.get("tool_input")
        if not isinstance(tool_input, dict):
            return _deny(event)
        return (
            b""
            if _nonblank_string(tool_input, "model")
            and _nonblank_string(tool_input, "thinking")
            else _deny(event)
        )
    if tool in GITHUB_TOOLS:
        tool_input = payload.get("tool_input")
        if not isinstance(tool_input, dict):
            return _deny(event)
        repository = tool_input.get("repository_full_name")
        if isinstance(repository, str) and repository != CODEXY_REPOSITORY:
            return b""
        if repository != CODEXY_REPOSITORY:
            return _deny(event)
        return _deny(event) if _github_forbidden(tool, tool_input) else b""
    if tool != "Bash":
        return b""
    tool_input = payload.get("tool_input")
    cwd = payload.get("cwd")
    if not isinstance(tool_input, dict) or not isinstance(cwd, str):
        return _deny(event)
    owned = repository_owned(cwd)
    if owned is False:
        return b""
    command = tool_input.get("command")
    if owned is None or not isinstance(command, str):
        return _deny(event)
    return _deny(event) if shell_forbidden(command) else b""


def evaluate(event: Event, payload: bytes) -> bytes:
    """Return a fixed event-native denial or a zero-byte pass-through receipt."""
    if len(payload) > MAX_INPUT_BYTES:
        return _deny(event)
    try:
        decoded = payload.decode("utf-8", errors="strict")
        data = json.loads(decoded, object_pairs_hook=_unique_object)
    except (UnicodeError, json.JSONDecodeError, DuplicateKey, ValueError):
        return _deny(event)
    if not isinstance(data, dict):
        return _deny(event)
    return _evaluate_object(event, data)


def main() -> int:
    parser = argparse.ArgumentParser(prog="codexy-hook-policy", allow_abbrev=False)
    subcommands = parser.add_subparsers(dest="command", required=True)
    evaluate_parser = subcommands.add_parser("evaluate", allow_abbrev=False)
    evaluate_parser.add_argument("--event", choices=("PreToolUse", "PermissionRequest"), required=True)
    arguments = parser.parse_args()
    payload = sys.stdin.buffer.read(MAX_INPUT_BYTES + 1)
    output = evaluate(arguments.event, payload)
    if output:
        sys.stdout.buffer.write(output)
    return 0
