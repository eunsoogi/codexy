"""Enforce the assigned Worker route and app-worktree creation environment."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import cast

from .child_thread_creation_feedback import (
    app_worktree_denial,
    missing_field_denial,
    missing_route_denial,
    parent_checkout_denial,
    parent_worktree_denial,
    unsupported_model_denial,
    unsupported_thinking_denial,
)
from .envelope import Diagnostic, Request
from .thread_delivery import FIELDS, WORKER_CREATION_ROUTES


def forbidden(request: Request) -> bool | Diagnostic:
    checkout_diagnostic = _parent_checkout_diagnostic(request.cwd)
    if checkout_diagnostic is not None:
        return checkout_diagnostic

    tool_input = request.tool_input
    if not isinstance(tool_input, dict):
        return missing_route_denial()
    data = cast(dict[str, object], tool_input)
    if not _is_app_worktree_target(data.get("target")):
        return app_worktree_denial()
    missing = [field for field in FIELDS if not _non_empty_string(data.get(field))]
    if missing:
        return missing_field_denial(missing, data)
    model = cast(str, data["model"])
    thinking = cast(str, data["thinking"])
    # Match the shared allowlist as pairs, so a model cannot borrow another pair's setting.
    if not any(model == route_model for route_model, _ in WORKER_CREATION_ROUTES):
        return unsupported_model_denial()
    if (model, thinking) not in WORKER_CREATION_ROUTES:
        return unsupported_thinking_denial(data)
    return False


def _is_app_worktree_target(target: object) -> bool:
    # Require the managed worktree at creation time; a later directory change is not equivalent.
    if not isinstance(target, dict):
        return False
    target_data = cast(dict[str, object], target)
    if target_data.get("type") != "project" or not _non_empty_string(
        target_data.get("projectId")
    ):
        return False
    environment = target_data.get("environment")
    if not isinstance(environment, dict):
        return False
    environment_data = cast(dict[str, object], environment)
    return environment_data.get("type") == "worktree"


def _parent_checkout_diagnostic(cwd: object) -> Diagnostic | None:
    if not isinstance(cwd, str) or not cwd.strip():
        return parent_checkout_denial()

    try:
        actual = Path(cwd).resolve(strict=True)
    except (OSError, RuntimeError, ValueError):
        return parent_checkout_denial()
    if not actual.is_dir():
        return parent_checkout_denial()

    try:
        checkout = subprocess.run(
            ["git", "-C", str(actual), "rev-parse", "--show-toplevel"],
            check=False,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=3,
        )
        result = subprocess.run(
            ["git", "-C", str(actual), "worktree", "list", "--porcelain"],
            check=False,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=3,
        )
    except (OSError, subprocess.SubprocessError, ValueError):
        return parent_checkout_denial()
    if checkout.returncode != 0 or result.returncode != 0:
        return parent_checkout_denial()

    blocks = [block for block in result.stdout.split("\n\n") if block.strip()]
    if not blocks or "bare" in blocks[0].splitlines():
        return parent_checkout_denial()

    try:
        checkout_root = Path(checkout.stdout.strip()).resolve(strict=True)
    except (OSError, RuntimeError, ValueError):
        return parent_checkout_denial()

    worktrees = [
        Path(line.removeprefix("worktree ")).resolve(strict=False)
        for line in result.stdout.splitlines()
        if line.startswith("worktree ")
    ]
    if not worktrees:
        return parent_checkout_denial()

    # Git lists the primary checkout first; linked worktree CWDs are blocked before child creation.
    primary, *linked = worktrees
    if checkout_root == primary:
        return None
    if checkout_root in linked:
        return parent_worktree_denial()
    return parent_checkout_denial()


def _non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip()) and value == value.strip()
