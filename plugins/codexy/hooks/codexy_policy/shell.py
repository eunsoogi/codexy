"""Concern-specific structural checks for sensitive shell operations."""

from __future__ import annotations

from .execution_context import ExecutionContext
from .repository import git_directory_owned, repository_owned
from .shell_destructive_policy import POLICY as DESTRUCTIVE_POLICY
from .shell_evaluator import evaluate
from .shell_github_policy import POLICY as GITHUB_POLICY


def github_forbidden(
    command: str, cwd: str, gh_repo: str | None = None, git_dir: str | None = None,
    git_common_dir: str | None = None,
    git_config_environment: tuple[tuple[str, str], ...] = (), depth: int = 0,
    runtime_environment: tuple[tuple[str, str], ...] = (),
) -> bool:
    return _entry(
        command, cwd, gh_repo, git_dir, git_common_dir,
        git_config_environment, depth, runtime_environment, GITHUB_POLICY,
    )


def destructive_forbidden(
    command: str, cwd: str, gh_repo: str | None = None, git_dir: str | None = None,
    git_common_dir: str | None = None,
    git_config_environment: tuple[tuple[str, str], ...] = (), depth: int = 0,
    runtime_environment: tuple[tuple[str, str], ...] = (),
) -> bool:
    return _entry(
        command, cwd, gh_repo, git_dir, git_common_dir,
        git_config_environment, depth, runtime_environment, DESTRUCTIVE_POLICY,
    )


def _entry(
    command: str, cwd: str, gh_repo: str | None, git_dir: str | None,
    git_common_dir: str | None,
    git_config_environment: tuple[tuple[str, str], ...], depth: int,
    runtime_environment: tuple[tuple[str, str], ...], policy: object,
) -> bool:
    environment = runtime_environment + tuple(
        (key, value)
        for key, value in (
            ("GH_REPO", gh_repo),
            ("GIT_DIR", git_dir),
            ("GIT_COMMON_DIR", git_common_dir),
        )
        if value is not None
    ) + git_config_environment
    owned = git_directory_owned(cwd, git_dir) if git_dir is not None else repository_owned(cwd)
    context = ExecutionContext(
        cwd, owned, git_dir, gh_repo, environment,
        opaque_repository_state=git_common_dir is not None,
    )
    return evaluate(command, context, depth, policy)
