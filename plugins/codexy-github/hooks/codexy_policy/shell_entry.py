"""Concern-neutral construction of shell evaluation context."""

from __future__ import annotations

from .execution_context import ExecutionContext
from .repository import (
    git_directory_owned,
    repository_identity,
    repository_owned,
    repository_policy_status,
)


def context(
    cwd: str,
    gh_repo: str | None,
    git_dir: str | None,
    git_common_dir: str | None,
    git_config_environment: tuple[tuple[str, str], ...],
    runtime_environment: tuple[tuple[str, str], ...],
) -> ExecutionContext:
    environment = (
        runtime_environment
        + tuple(
            (key, value)
            for key, value in (
                ("GH_REPO", gh_repo),
                ("GIT_DIR", git_dir),
                ("GIT_COMMON_DIR", git_common_dir),
            )
            if value is not None
        )
        + git_config_environment
    )
    owned = (
        git_directory_owned(cwd, git_dir)
        if git_dir is not None
        else repository_owned(cwd)
    )
    return ExecutionContext(
        cwd,
        owned,
        repository_policy_status(cwd),
        repository_identity(cwd),
        git_dir,
        gh_repo,
        environment,
        opaque_repository_state=git_common_dir is not None,
    )
