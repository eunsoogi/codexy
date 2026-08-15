"""Immutable state types used by shell execution policy."""

from __future__ import annotations

from dataclasses import dataclass

from .filesystem_state import PathState


@dataclass(frozen=True)
class ExecutionContext:
    cwd: str
    cwd_owned: bool | None
    policy_status: bool | None
    policy_identity: tuple[str, str, str] | None
    git_dir: str | None
    gh_repo: str | None
    environment: tuple[tuple[str, str], ...] = ()
    opaque_environment: bool = False
    remote_urls: tuple[tuple[str, str, str], ...] = ()
    opaque_repository_state: bool = False
    executable_aliases: tuple[tuple[str, PathState], ...] = ()
    opaque_filesystem_state: bool = False


@dataclass(frozen=True)
class CommandEffect:
    success: ExecutionContext | None
    failure: ExecutionContext | None = None
