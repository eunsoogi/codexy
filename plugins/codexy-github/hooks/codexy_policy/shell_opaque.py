"""Concern-neutral decisions over parsed shell command positions."""

from __future__ import annotations

import re
from dataclasses import dataclass

from .execution_context import ExecutionContext, assignment
from .invocation import Invocation, resolve as resolve_invocation
from .shell_segments import command_tokens, segments, separate_lines

DYNAMIC_NAME = re.compile(r"\$(?:\{[A-Za-z_][A-Za-z0-9_]*\}|[A-Za-z_][A-Za-z0-9_]*)")
WRAPPER_HEADS = frozenset(
    {
        "builtin", "command", "coproc", "env", "exec", "nice", "nohup", "sudo",
        "time", "timeout", "xargs",
    }
)


@dataclass(frozen=True)
class ResolvedSegment:
    """One parsed command-position segment and its effective invocation."""

    tokens: tuple[str, ...]
    command: tuple[str, ...]
    invocation: Invocation | None


def resolved_segments(
    command: str, context: ExecutionContext
) -> tuple[ResolvedSegment, ...] | None:
    """Use one shared walk for every protected-effect classifier."""
    parsed = segments(command)
    if parsed is None:
        return None
    return tuple(
        ResolvedSegment(tokens, command_tokens(tokens), resolve_invocation(list(tokens), context))
        for tokens in parsed
    )


def dynamic_control_executable(command: str) -> bool:
    """Fail closed only for a dynamic token in command position."""
    parsed = segments(command)
    return parsed is None or any(
        command_tokens(segment) and DYNAMIC_NAME.fullmatch(command_tokens(segment)[0])
        for segment in parsed
    )


def unresolved_protected_effect(command: str, context: ExecutionContext) -> bool:
    """Recognize command-position indirection that can conceal an effect."""
    walked = resolved_segments(command, context)
    if walked is None:
        return False
    for segment in walked:
        invocation = segment.invocation
        if invocation is None:
            continue
        if (
            invocation.executable == "eval"
            and (
                invocation.opaque
                or not invocation.arguments
                or any(DYNAMIC_NAME.fullmatch(argument) for argument in invocation.arguments)
            )
        ) or _dynamic_path_target(segment.tokens):
            return True
        if invocation.script and unresolved_protected_effect(
            invocation.script, invocation.context
        ):
            return True
    return False


def contains_policy_executable(
    command: str, context: ExecutionContext, expected: str
) -> bool:
    """Find a protected executable or unresolved command target structurally."""
    walked = resolved_segments(command, context)
    if walked is None:
        return True
    for segment in walked:
        invocation = segment.invocation
        if invocation is not None and invocation.script and contains_policy_executable(
            invocation.script, invocation.context, expected
        ):
            return True
        if invocation is not None and invocation.executable == "eval" and (
            contains_policy_executable(
                " ".join(invocation.arguments), invocation.context, expected
            )
            if invocation.arguments
            else True
        ):
            return True
        if invocation is not None and invocation.executable == expected:
            return True
        if invocation is not None and unresolved_invocation(invocation):
            return True
    return False


def unresolved_invocation(invocation: Invocation) -> bool:
    """Return whether an already parsed invocation lacks an effective target."""
    return invocation.opaque and (
        invocation.executable is None or invocation.executable in WRAPPER_HEADS
    )


def unresolved_alias_transition(command: str, context: ExecutionContext) -> bool:
    """Fail closed only when a dynamic alias is later executed as its target."""
    walked = resolved_segments(command, context)
    if walked is None:
        return True
    if any(_forced_opaque_alias(segment) for segment in walked):
        return True
    destinations = {
        destination
        for segment in walked
        if (destination := _opaque_alias_destination(segment)) is not None
    }
    return any(
        _target_path(segment.command[0], context.cwd) in destinations
        for segment in walked
        if segment.command
    )


def _opaque_alias_destination(segment: ResolvedSegment) -> str | None:
    invocation = segment.invocation
    if (
        invocation is None
        or not invocation.opaque
        or invocation.executable not in {"ln", "cp"}
    ):
        return None
    operands = [token for token in invocation.arguments if not token.startswith("-")]
    if len(operands) != 2 or DYNAMIC_NAME.search(operands[1]) is not None:
        return None
    return _target_path(operands[1], invocation.context.cwd)


def _forced_opaque_alias(segment: ResolvedSegment) -> bool:
    invocation = segment.invocation
    return (
        invocation is not None
        and invocation.opaque
        and invocation.executable in {"ln", "cp"}
        and any(
            argument == "--force"
            or (argument.startswith("-") and not argument.startswith("--") and "f" in argument[1:])
            for argument in invocation.arguments
        )
    )


def _target_path(value: str, cwd: str) -> str:
    from os.path import abspath, join

    return abspath(value if value.startswith("/") else join(cwd, value))


def _dynamic_path_target(tokens: tuple[str, ...]) -> bool:
    index = 0
    while index < len(tokens) and (tokens[index] == "!" or assignment(tokens[index])):
        index += 1
    return (
        index < len(tokens)
        and not tokens[index].startswith("/")
        and any(
            token.startswith("PATH=") and DYNAMIC_NAME.search(token) is not None
            for token in tokens[:index]
        )
    )
