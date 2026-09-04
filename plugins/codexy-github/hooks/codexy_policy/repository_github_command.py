"""Owned repository GitHub CLI/API command admission."""

import os

from .envelope import Request
from .policy_diagnostics import describe
from .shell_entry import context as shell_context
from .shell_github import forbidden as shell_forbidden
from .shell_evaluator import credential_exposure
from .shell_opaque import unresolved_protected_effect


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
    if credential_exposure(command, context):
        return describe("CREDENTIAL_EXPOSURE", command, context)
    if unresolved_protected_effect(command, context):
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
        return describe("REMOTE_MUTATION", command, context)
    return False
