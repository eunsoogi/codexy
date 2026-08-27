"""Concern-neutral shell parsing with an injected single-concern policy."""

from __future__ import annotations

import shlex
from pathlib import Path
from typing import Protocol

from .execution_context import (
    CommandEffect,
    ExecutionContext,
    after_external_command,
    assignment,
    at as context_at,
)
from .invocation import Invocation, resolve
from .shell_context import changed_directory
from .shell_groups import GroupSyntaxError, parse
from .shell_opaque import dynamic_control_executable, resolved_segments, separate_lines
from .shell_segments import opaque_syntax, segments
from .shell_sequence import evaluate as evaluate_sequence


class Policy(Protocol):
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
    if syntax.substitutions or syntax.control:
        for nested in syntax.substitutions:
            if evaluate(nested, context, depth + 1, policy):
                return True
        lexical_command = syntax.command
    try:
        lexer = shlex.shlex(
            separate_lines(lexical_command), posix=True, punctuation_chars=";&|(){}"
        )
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
    except ValueError:
        return context.cwd_owned is not False and policy.owns_opaque(command, context)
    try:
        sequence = parse(tokens)
    except GroupSyntaxError:
        if syntax.control:
            return dynamic_control_executable(command) or _control_segments(
                command, context, depth, policy
            )
        return context.cwd_owned is not False and policy.owns_opaque(command, context)
    return evaluate_sequence(
        sequence,
        context,
        depth,
        lambda tokens, current, current_depth: _segment(
            tokens, current, current_depth, policy
        ),
    )[0]


class _CredentialPolicy:
    """Detect a credential operation through the ordinary stateful effect walk."""

    @staticmethod
    def owns_opaque(command: str, context: ExecutionContext) -> bool:
        return False

    @staticmethod
    def opaque_invocation(invocation: Invocation) -> bool:
        return False

    @staticmethod
    def command(
        invocation: Invocation, outer: ExecutionContext, depth: int
    ) -> tuple[bool, CommandEffect] | None:
        if _credential_environment(invocation.context):
            return True, CommandEffect(None)
        if invocation.executable != "gh":
            return None
        return (
            invocation.arguments[:2] == ["auth", "token"]
            or _auth_status_exposes_token(invocation.arguments)
            or _credential_header(invocation.arguments),
            CommandEffect(outer),
        )


def credential_exposure(
    command: str, context: ExecutionContext, depth: int = 0
) -> bool:
    walked = resolved_segments(command, context)
    if walked is not None and any(
        _credential_assignment(
            segment.tokens[: len(segment.tokens) - len(segment.command)]
        )
        for segment in walked
    ):
        return True
    return evaluate(command, context, depth, _CredentialPolicy())


def _credential_assignment(tokens: tuple[str, ...]) -> bool:
    return any(
        assignment(token)
        and token.split("=", 1)[0]
        in {
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "GH_ENTERPRISE_TOKEN",
            "GITHUB_ENTERPRISE_TOKEN",
        }
        and bool(token.split("=", 1)[1])
        for token in tokens
    )


def _credential_environment(context: ExecutionContext) -> bool:
    return _credential_assignment(
        tuple(f"{key}={value}" for key, value in context.environment)
    )


def _credential_header(arguments: list[str]) -> bool:
    for index, argument in enumerate(arguments):
        header = (
            arguments[index + 1]
            if argument in {"-H", "--header"} and index + 1 < len(arguments)
            else argument.split("=", 1)[1]
            if argument.startswith(("-H=", "--header="))
            else None
        )
        if header is None:
            continue
        name, separator, value = header.partition(":")
        if (
            separator
            and name.casefold() in {"authorization", "x-github-token"}
            and value.strip()
        ):
            return True
    return False


def _auth_status_exposes_token(arguments: list[str]) -> bool:
    return arguments[:2] == ["auth", "status"] and any(
        option in {"--show-token", "--with-token"} for option in arguments[2:]
    )


def _segment(
    tokens: list[str],
    context: ExecutionContext,
    depth: int,
    policy: Policy,
) -> tuple[bool, CommandEffect]:
    invocation = resolve(tokens, context, depth)
    if invocation is None:
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
        return False, _test_effect(invocation.arguments, context)
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
    command: str, context: ExecutionContext, depth: int, policy: Policy
) -> bool:
    """Walk parsed control bodies through the same typed invocation classifier."""
    parsed = segments(command)
    if parsed is None:
        return False
    current = context
    for tokens in parsed:
        denied, effect = _segment(list(tokens), current, depth + 1, policy)
        if denied:
            return True
        current = effect.success or effect.failure or current
    return False


def _test_effect(arguments: list[str], context: ExecutionContext) -> CommandEffect:
    if len(arguments) != 2 or arguments[0] != "-e":
        return CommandEffect(context, context)
    path = Path(arguments[1])
    candidate = path if path.is_absolute() else Path(context.cwd) / path
    return (
        CommandEffect(context) if candidate.exists() else CommandEffect(None, context)
    )
