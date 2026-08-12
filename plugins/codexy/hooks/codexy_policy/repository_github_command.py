"""Owned repository GitHub CLI/API command admission."""

import os

from .envelope import Request
from .shell_github import forbidden as shell_forbidden


def forbidden(request: Request) -> bool:
    data = request.tool_input
    if (
        not isinstance(data, dict)
        or not isinstance(data.get("command"), str)
        or not isinstance(request.cwd, str)
    ):
        return True
    return shell_forbidden(
        data["command"],
        request.cwd,
        os.environ.get("GH_REPO") or None,
        os.environ.get("GIT_DIR") or None,
        os.environ.get("GIT_COMMON_DIR") or None,
        tuple((key, value) for key, value in os.environ.items() if key.startswith("GIT_CONFIG_")),
        runtime_environment=tuple(
            (key, os.environ.get(key, "")) for key in ("HOME", "PATH", "USER")
        ),
    )
