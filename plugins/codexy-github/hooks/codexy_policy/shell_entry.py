"""Concern-neutral construction of shell evaluation context."""

from __future__ import annotations

from pathlib import Path

from .execution_context import ExecutionContext
from .repository import git_directory_owned, repository_owned, repository_status


def context(
    cwd: str,
    gh_repo: str | None,
    git_dir: str | None,
    git_common_dir: str | None,
    git_config_environment: tuple[tuple[str, str], ...],
    runtime_environment: tuple[tuple[str, str], ...],
) -> ExecutionContext:
    # The hook runner's PWD may differ; only the request's absolute CWD describes this shell.
    shell_environment = tuple(
        (key, value) for key, value in runtime_environment if key != "PWD"
    )
    if Path(cwd).is_absolute():
        shell_environment += (("PWD", cwd),)
    # Seed the evaluator with only the host variables its repository and Git policy understands.
    environment = (
        shell_environment
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
    # A common-dir override makes repository identity opaque until later commands re-establish it.
    return ExecutionContext(
        cwd,
        owned,
        repository_status(cwd),
        git_dir,
        gh_repo,
        environment,
        opaque_repository_state=git_common_dir is not None,
    )
