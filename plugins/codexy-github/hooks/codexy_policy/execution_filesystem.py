"""External filesystem effects for shell execution policy."""

from __future__ import annotations

from dataclasses import replace
from pathlib import Path

from .executable_identity import alias_transition
from .filesystem_state import FAILURE, mkdir, replace_path_state
from .execution_context_types import CommandEffect, ExecutionContext


def after_external_command(
    executable: str, arguments: list[str], context: ExecutionContext
) -> CommandEffect | None:
    """Apply bounded external filesystem and Git-config state transitions."""
    if executable == "mkdir":
        return mkdir_effect(arguments, context)
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
