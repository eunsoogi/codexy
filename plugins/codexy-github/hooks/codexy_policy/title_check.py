"""Title-only GitHub checks for direct, shell, and nested tool surfaces."""

from __future__ import annotations

from collections.abc import Mapping
from typing import cast

from .envelope import Request
from .title_nested import forbidden as nested_forbidden
from .title_shell import forbidden as shell_forbidden
from .titles import issue_title, pr_title, squash_subject


ISSUE_TOOLS = frozenset(
    {
        "mcp__codex_apps__github_create_issue",
        "mcp__codex_apps__github_update_issue",
        "github.create_issue",
        "github.update_issue",
        "github_create_issue",
        "github_update_issue",
    }
)
PR_TOOLS = frozenset(
    {
        "mcp__codex_apps__github_create_pull_request",
        "mcp__codex_apps__github_update_pull_request",
        "github.create_pull_request",
        "github.update_pull_request",
        "github_create_pull_request",
        "github_update_pull_request",
    }
)
MERGE_TOOLS = frozenset(
    {
        "mcp__codex_apps__github_merge_pull_request",
        "mcp__codex_apps__github_enable_auto_merge",
        "github.merge_pull_request",
        "github.enable_auto_merge",
        "github_merge_pull_request",
        "github_enable_auto_merge",
    }
)
TOOLS = {
    "issue": ISSUE_TOOLS,
    "pr": PR_TOOLS,
    "merge": MERGE_TOOLS,
    "shell": frozenset({"Bash"}),
    "nested": frozenset({"functions.exec"}),
}


def forbidden(request: Request, kind: str) -> bool:
    if kind in {"issue", "pr"}:
        return _direct(request, kind)
    if kind == "merge":
        return _merge(request)
    if kind == "shell":
        data = _mapping(request.tool_input)
        return data is not None and shell_forbidden(data.get("command"))
    if kind == "nested":
        data = _mapping(request.tool_input)
        return data is not None and nested_forbidden(data.get("code"))
    return False


def _direct(request: Request, kind: str) -> bool:
    data = _mapping(request.tool_input)
    if data is None:
        return True
    operation = _operation(request.tool)
    create_operation, predicate = {
        "issue": ("create_issue", issue_title),
        "pr": ("create_pull_request", pr_title),
    }[kind]
    update_operation = create_operation.replace("create", "update", 1)
    if operation == create_operation:
        value = data.get("title")
        return not isinstance(value, str) or not predicate(value)
    if operation == update_operation:
        value = data.get("title")
        return value is not None and (
            not isinstance(value, str) or not predicate(value)
        )
    return False


def _merge(request: Request) -> bool:
    if request.tool not in MERGE_TOOLS:
        return False
    data = _mapping(request.tool_input)
    if data is None:
        return True
    if (
        _operation(request.tool) != "merge_pull_request"
        or data.get("merge_method") != "squash"
    ):
        return False
    return not squash_subject(data.get("commit_title"), data.get("pr_number"))


def _operation(tool: str) -> str:
    value = tool.removeprefix("mcp__codex_apps__")
    value = value.removeprefix("github.")
    return value.removeprefix("github_")


def _mapping(value: object) -> Mapping[str, object] | None:
    return cast("Mapping[str, object]", value) if isinstance(value, dict) else None
