"""Destructive-command shell admission entrypoint."""

from __future__ import annotations

from .shell_destructive_policy import POLICY
from .shell_entry import context
from .shell_evaluator import evaluate


def forbidden(
    command: str, cwd: str, gh_repo: str | None = None, git_dir: str | None = None,
    git_common_dir: str | None = None,
    git_config_environment: tuple[tuple[str, str], ...] = (), depth: int = 0,
    runtime_environment: tuple[tuple[str, str], ...] = (),
) -> bool:
    return evaluate(
        command,
        context(
            cwd, gh_repo, git_dir, git_common_dir,
            git_config_environment, runtime_environment,
        ),
        depth,
        POLICY,
    )
