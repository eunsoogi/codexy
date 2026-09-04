"""Destructive shell, filesystem, and Git command admission."""

import os

from .envelope import Request
from .policy_diagnostics import describe
from .shell_entry import context as shell_context
from .shell_destructive import forbidden as shell_forbidden
from .shell_destructive_policy import POLICY
from .shell_opaque import unresolved_alias_transition, unresolved_protected_effect


def forbidden(request: Request) -> bool | str:
    data = request.tool_input
    if (
        not isinstance(data, dict)
        or not isinstance(data.get("command"), str)
        or not isinstance(request.cwd, str)
    ):
        return describe("UNRESOLVED_TARGET")
    command = data["command"]
    runtime_environment = tuple(
        (key, os.environ.get(key, "")) for key in ("HOME", "PATH", "USER")
    )
    git_config_environment = tuple(
        (key, value)
        for key, value in os.environ.items()
        if key.startswith("GIT_CONFIG_")
    )
    context = shell_context(
        request.cwd,
        os.environ.get("GH_REPO") or None,
        os.environ.get("GIT_DIR") or None,
        os.environ.get("GIT_COMMON_DIR") or None,
        git_config_environment,
        runtime_environment,
    )
    if unresolved_protected_effect(command, context) or unresolved_alias_transition(
        command, context
    ):
        return describe("UNRESOLVED_PROTECTED_EFFECT", command, context)
    if shell_forbidden(
        command,
        request.cwd,
        os.environ.get("GH_REPO") or None,
        os.environ.get("GIT_DIR") or None,
        os.environ.get("GIT_COMMON_DIR") or None,
        git_config_environment,
        runtime_environment=runtime_environment,
    ):
        return describe("DESTRUCTIVE_EFFECT", command, context, policy=POLICY)
    return False
