"""Concern-neutral shell parsing with an injected single-concern policy."""

from __future__ import annotations

from dataclasses import replace
from itertools import count
from typing import Protocol

from .execution_context import at as context_at
from .execution_context_types import CommandEffect, ExecutionContext
from .execution_filesystem import after_external_command
from .invocation import Invocation, resolve
from .shell_builtins import test_effect
from .shell_context import changed_directory
from .shell_credentials import CredentialPolicy, credential_assignment
from .shell_groups import GroupSyntaxError, parse
from .shell_opaque import DYNAMIC_NAME
from .shell_redirections import UNSAFE_REDIRECTION
from .shell_segments import (
    command_tokens,
    opaque_syntax,
    segments,
    tokenize,
)
from .shell_sequence import evaluate as evaluate_sequence


class Policy(Protocol):
    redirection_executables: frozenset[str]

    def owns_opaque(self, command: str, context: ExecutionContext) -> bool: ...
    def opaque_invocation(self, invocation: Invocation) -> bool: ...
    def command(
        self, invocation: Invocation, outer: ExecutionContext, depth: int
    ) -> tuple[bool, CommandEffect] | None: ...


def evaluate(
    command: str, context: ExecutionContext, depth: int, policy: Policy
) -> bool:
    lexical_command = command
    syntax = opaque_syntax(command)
    restrict_nested_temp_output = context.restrict_private_temp_output or bool(
        syntax.substitutions
    )
    # Inspect substitutions before simplifying syntax and retain their state for redirect checks.
    if syntax.substitutions or syntax.control:
        nested_context = replace(
            context, restrict_private_temp_output=restrict_nested_temp_output
        )
        for nested in syntax.substitutions:
            if evaluate(nested, nested_context, depth + 1, policy):
                return True
        lexical_command = syntax.command
    tokens = tokenize(lexical_command, restrict_nested_temp_output)
    if tokens is None:
        return context.cwd_owned is not False and policy.owns_opaque(command, context)
    try:
        sequence = parse(tokens)
    except GroupSyntaxError:
        if syntax.control:  # Fall back for dynamic heads in partial syntax.
            parsed = segments(command)
            if parsed is None:
                return True
            return any(
                command_tokens(segment)
                and DYNAMIC_NAME.fullmatch(command_tokens(segment)[0])
                for segment in parsed
            ) or _control_segments(
                parsed, context, depth, policy, restrict_nested_temp_output
            )
        return context.cwd_owned is not False and policy.owns_opaque(command, context)
    segments_seen = count()
    return evaluate_sequence(
        sequence,
        context,
        depth,
        lambda tokens, current, current_depth: _segment(
            tokens,
            current,
            current_depth,
            policy,
            restrict_nested_temp_output or next(segments_seen) > 0,
        ),
    )[0]


def credential_exposure(
    command: str, context: ExecutionContext, depth: int = 0
) -> bool:
    return evaluate(command, context, depth, CredentialPolicy())


def _segment(
    tokens: list[str],
    context: ExecutionContext,
    depth: int,
    policy: Policy,
    restrict_private_temp_output: bool,
) -> tuple[bool, CommandEffect]:
    # Each segment returns explicit success/failure contexts so later shell operators see prior effects.
    if getattr(policy, "detect_leading_credentials", False):
        command_start = command_tokens(tuple(tokens))
        if credential_assignment(tokens[: len(tokens) - len(command_start)]):
            return True, CommandEffect(None)
    invocation = resolve(
        [token for token in tokens if token != UNSAFE_REDIRECTION], context, depth
    )
    if invocation is None:
        return True, CommandEffect(None)
    if (
        restrict_private_temp_output
        and not invocation.context.restrict_private_temp_output
    ):
        invocation = replace(
            invocation,
            context=replace(invocation.context, restrict_private_temp_output=True),
        )
    if (
        UNSAFE_REDIRECTION in tokens
        and invocation.executable in policy.redirection_executables
    ):
        return True, CommandEffect(None)
    if invocation.script is not None:
        return not invocation.script or evaluate(
            invocation.script, invocation.context, depth + 1, policy
        ), CommandEffect(context)
    if invocation.opaque:
        if policy.opaque_invocation(invocation):
            return True, CommandEffect(None)
        result = policy.command(invocation, context, depth)
        if result is not None:
            return result
        return False, CommandEffect(None)
    if invocation.executable is None:
        return False, CommandEffect(invocation.context)
    if invocation.executable == "eval":
        return evaluate(
            " ".join(invocation.arguments), invocation.context, depth + 1, policy
        ), CommandEffect(context)
    if invocation.executable == "false":
        return False, CommandEffect(None, context)
    if invocation.executable == "true":
        return False, CommandEffect(context)
    if invocation.executable == "test":
        return False, test_effect(invocation.arguments, context)
    if invocation.executable in {"cd", "pushd", "popd"}:
        directory = changed_directory(
            [invocation.executable, *invocation.arguments], invocation.context.cwd
        )
        return (
            (True, CommandEffect(None))
            if directory.opaque
            else (False, CommandEffect(context_at(invocation.context, directory.cwd)))
        )
    if invocation.executable in {".", "source"}:
        return True, CommandEffect(None)
    result = policy.command(invocation, context, depth)
    if result is not None:
        return result
    effect = after_external_command(
        invocation.executable,
        invocation.arguments,
        context,
    )
    return (True, CommandEffect(None)) if effect is None else (False, effect)


def _control_segments(
    parsed: tuple[tuple[str, ...], ...],
    context: ExecutionContext,
    depth: int,
    policy: Policy,
    restrict_nested_temp_output: bool,
) -> bool:
    """Walk parsed control bodies through the same typed invocation classifier."""
    current, segments_seen = context, count()
    for tokens in parsed:
        denied, effect = _segment(
            list(tokens),
            current,
            depth + 1,
            policy,
            restrict_nested_temp_output or next(segments_seen) > 0,
        )
        if denied:
            return True
        current = effect.success or effect.failure or current
    return False
