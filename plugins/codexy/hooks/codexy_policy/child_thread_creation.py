"""Enforce the assigned Worker route for native thread creation."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import cast

from .envelope import Diagnostic, Request
from .thread_delivery import FIELDS, WORKER_ROUTE

# Native ordinary-route creation is the assigned Worker route. The hook filename
# remains a compatibility identifier for the installed concern and launcher.
WORKER_MODEL, WORKER_THINKING = WORKER_ROUTE


def forbidden(request: Request) -> bool | Diagnostic:
    checkout_diagnostic = _parent_checkout_diagnostic(request.cwd)
    if checkout_diagnostic is not None:
        return checkout_diagnostic

    tool_input = request.tool_input
    if not isinstance(tool_input, dict):
        return Diagnostic("MISSING_ROUTE_FIELDS", _MISSING_FIELDS)
    data = cast(dict[str, object], tool_input)
    missing = [field for field in FIELDS if not _non_empty_string(data.get(field))]
    if missing:
        return _missing_field_diagnostic(missing)
    if data["model"] != WORKER_MODEL:
        return Diagnostic("UNSUPPORTED_MODEL", _UNSUPPORTED_MODEL)
    if data["thinking"] != WORKER_THINKING:
        return Diagnostic("UNSUPPORTED_THINKING", _UNSUPPORTED_THINKING)
    return False


def _parent_checkout_diagnostic(cwd: object) -> Diagnostic | None:
    if not isinstance(cwd, str) or not cwd.strip():
        return Diagnostic("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)

    try:
        actual = Path(cwd).resolve(strict=True)
    except (OSError, RuntimeError, ValueError):
        return Diagnostic("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)
    if not actual.is_dir():
        return Diagnostic("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)

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
        return Diagnostic("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)
    if checkout.returncode != 0 or result.returncode != 0:
        return Diagnostic("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)

    blocks = [block for block in result.stdout.split("\n\n") if block.strip()]
    if not blocks or "bare" in blocks[0].splitlines():
        return Diagnostic("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)

    try:
        checkout_root = Path(checkout.stdout.strip()).resolve(strict=True)
    except (OSError, RuntimeError, ValueError):
        return Diagnostic("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)

    worktrees = [
        Path(line.removeprefix("worktree ")).resolve(strict=False)
        for line in result.stdout.splitlines()
        if line.startswith("worktree ")
    ]
    if not worktrees:
        return Diagnostic("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)

    primary, *linked = worktrees
    if checkout_root == primary:
        return None
    if checkout_root in linked:
        return Diagnostic("PARENT_WORKTREE", _PARENT_WORKTREE)
    return Diagnostic("PARENT_CHECKOUT_UNVERIFIED", _UNVERIFIED_PARENT)


def _non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip()) and value == value.strip()


def _missing_field_diagnostic(fields: list[str]) -> Diagnostic:
    names = " and ".join(fields)
    code = "MISSING_ROUTE_FIELDS" if len(fields) > 1 else f"MISSING_{fields[0].upper()}"
    return Diagnostic(
        code,
        f"Missing {names}; {_REQUIRED_ROUTE}. MUST correct the field and MUST retry once.",
    )


_REQUIRED_ROUTE = "Worker creation requires model='gpt-6-luna' and thinking='max'"
_MISSING_FIELDS = (
    f"Missing model and thinking; {_REQUIRED_ROUTE}. "
    "MUST correct the fields and MUST retry once."
)
_UNSUPPORTED_MODEL = (
    f"Unsupported Worker creation model; {_REQUIRED_ROUTE}. "
    "MUST NOT substitute another model or silently fall back."
)
_UNSUPPORTED_THINKING = (
    f"Unsupported Worker creation thinking; {_REQUIRED_ROUTE}. "
    "MUST NOT substitute another reasoning setting or silently fall back."
)
_PARENT_WORKTREE = (
    "Child task creation is blocked because the actual parent CWD is in a linked "
    "Git worktree. Use a supported handoff to move the Orchestrator to the saved "
    "project's primary repository, then verify the destination CWD is inside the "
    "primary checkout before retrying. `handoff_thread` cannot move its own caller; "
    "a different controlling task MUST perform the handoff."
)
_UNVERIFIED_PARENT = (
    "Child task creation is blocked because the actual parent CWD could not be "
    "verified inside the saved project's primary Git checkout. Use a supported "
    "handoff to the primary repository, verify the destination CWD and checkout, "
    "then retry. `handoff_thread` cannot move its own caller; a different "
    "controlling task MUST perform the handoff."
)
