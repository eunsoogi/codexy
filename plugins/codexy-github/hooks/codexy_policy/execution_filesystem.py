"""External filesystem effects for shell execution policy."""

from __future__ import annotations

import os
import stat
from dataclasses import replace
from pathlib import Path, PurePosixPath

from .executable_identity import alias_transition
from .filesystem_state import FAILURE, mkdir, replace_path_state
from .execution_context_types import CommandEffect, ExecutionContext

_PRIVATE_TEMP = PurePosixPath("/private") / "tmp"


def safe_output_redirection(operator: str, value: str) -> bool:
    """Allow /dev/null or a direct, single-link file under macOS's private temp directory."""
    if value == "/dev/null":
        return True
    if operator != ">":
        return False
    path = PurePosixPath(value)
    if (
        path.parent != _PRIVATE_TEMP
        or path.name in {"", ".", ".."}
        or value != (_PRIVATE_TEMP / path.name).as_posix()
        or any(char in value for char in "$`*?[]\0")
    ):
        return False
    try:
        # Keep the host-specific directory out of distributable path literals.
        private_temp = Path(_PRIVATE_TEMP.as_posix())
        if private_temp.resolve().as_posix() != _PRIVATE_TEMP.as_posix():
            return False
        metadata = Path(value).lstat()
    except FileNotFoundError:
        return True
    except (OSError, RuntimeError, ValueError):
        return False
    # /private/tmp is shared, so an existing target must belong to this caller too.
    owner = getattr(os, "geteuid", None)
    return (
        owner is not None
        and stat.S_ISREG(metadata.st_mode)
        and metadata.st_nlink == 1
        and metadata.st_uid == owner()
    )


def after_external_command(
    executable: str, arguments: list[str], context: ExecutionContext
) -> CommandEffect | None:
    """Apply bounded external filesystem and Git-config state transitions."""
    if executable == "mkdir":
        return mkdir_effect(arguments, context)
    # Once filesystem state is opaque, later alias operations cannot be safely simulated.
    if context.opaque_filesystem_state and executable in {"ln", "cp"}:
        return None
    transition = alias_transition(
        executable, arguments, context.cwd, context.executable_aliases
    )
    if executable in {"ln", "cp"} and (transition is None or not transition.known):
        return None
    if transition is not None:
        aliases = replace_path_state(
            context.executable_aliases, transition.destination, transition.state
        )
        success = replace(context, executable_aliases=aliases)
        if transition.applies is True:
            return CommandEffect(success)
        if transition.applies is False:
            return CommandEffect(None, context)
        return CommandEffect(success, context)
    if executable != "sed" or not any(
        argument == "-i"
        or argument.startswith("-i")
        and len(argument) > 2
        or argument == "--in-place"
        or argument.startswith("--in-place=")
        for argument in arguments
    ):
        return CommandEffect(context, context)
    git_dir = Path(context.git_dir) if context.git_dir is not None else Path(".git")
    config = git_dir / "config"
    if not config.is_absolute():
        config = Path(context.cwd) / config
    target = config.resolve(strict=False)
    writes_config = any(
        not argument.startswith("-")
        and (
            Path(argument)
            if Path(argument).is_absolute()
            else Path(context.cwd) / argument
        ).resolve(strict=False)
        == target
        for argument in arguments
    )
    success = (
        replace(context, opaque_repository_state=True) if writes_config else context
    )
    return CommandEffect(success, context)


def mkdir_effect(
    arguments: list[str], context: ExecutionContext
) -> CommandEffect | None:
    outcome = mkdir(arguments, context.cwd, context.executable_aliases)
    if outcome.kind == "success":
        return CommandEffect(replace(context, executable_aliases=outcome.paths))
    if outcome.kind == FAILURE:
        return CommandEffect(None, context)
    return CommandEffect(replace(context, opaque_filesystem_state=True), context)
