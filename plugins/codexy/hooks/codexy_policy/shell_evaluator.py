"""Concern-neutral shell parsing with an injected single-concern policy."""

from __future__ import annotations

import re
import shlex
from pathlib import Path
from typing import Protocol

from .execution_context import (
    DYNAMIC_VALUE, SINGLE_QUOTED_DOLLAR, CommandEffect, ExecutionContext,
    after_external_command, at as context_at,
)
from .invocation import Invocation, resolve
from .shell_context import changed_directory
from .shell_git import explicit_owned
from .shell_groups import GroupSyntaxError, parse
from .shell_opaque import dynamic_control_executable, separate_lines
from .shell_sequence import evaluate as evaluate_sequence

OPAQUE = re.compile(r"\$\(|`|<<<?|\b(?:eval|if|for|while|until|case)\b")
SUBCOMMAND = re.compile(r"\$\(([^()]*)\)|`([^`]*)`")
CONTROL = re.compile(r"<<<?|\b(?:if|for|while|until|case)\b")


class Policy(Protocol):
    def owns_opaque(self, command: str, context: ExecutionContext) -> bool: ...
    def opaque_invocation(self, tokens: list[str], context: ExecutionContext) -> bool: ...
    def command(
        self, invocation: Invocation, outer: ExecutionContext, depth: int
    ) -> tuple[bool, CommandEffect] | None: ...


def evaluate(
    command: str, context: ExecutionContext, depth: int, policy: Policy
) -> bool:
    if depth > 3 or SINGLE_QUOTED_DOLLAR in command:
        return True
    lexical_command = command
    if OPAQUE.search(command):
        if context.cwd_owned is not False and policy.owns_opaque(command, context):
            return True
        try:
            opaque_tokens = shlex.split(command)
            if opaque_tokens and opaque_tokens[0].rsplit("/", 1)[-1].lower() == "eval":
                evaluated = opaque_tokens[1:]
                if evaluated[:1] == ["--"]:
                    evaluated = evaluated[1:]
                if evaluate(" ".join(evaluated), context, depth + 1, policy):
                    return True
            elif explicit_owned(opaque_tokens) is True:
                return True
        except ValueError:
            return True
        for match in SUBCOMMAND.finditer(command):
            nested = match.group(1) if match.group(1) is not None else match.group(2)
            if evaluate(nested, context, depth + 1, policy):
                return True
        lexical_command = SUBCOMMAND.sub(DYNAMIC_VALUE, command)
        if CONTROL.search(command):
            return policy.owns_opaque(command, context) or dynamic_control_executable(command)
    try:
        lexer = shlex.shlex(
            separate_lines(lexical_command), posix=True, punctuation_chars=";&|(){}"
        )
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
    except ValueError:
        return context.cwd_owned is not False
    try:
        sequence = parse(tokens)
    except GroupSyntaxError:
        return True
    return evaluate_sequence(
        sequence, context, depth,
        lambda tokens, current, current_depth: _segment(
            tokens, current, current_depth, policy
        ),
    )[0]


def _segment(
    tokens: list[str], context: ExecutionContext, depth: int, policy: Policy,
) -> tuple[bool, CommandEffect]:
    invocation = resolve(tokens, context, depth)
    if invocation is None:
        return True, CommandEffect(None)
    if invocation.script is not None:
        return not invocation.script or evaluate(
            invocation.script, invocation.context, depth + 1, policy
        ), CommandEffect(context)
    if invocation.opaque:
        return policy.opaque_invocation(tokens, invocation.context), CommandEffect(None)
    if invocation.executable is None:
        return False, CommandEffect(invocation.context)
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
        return (True, CommandEffect(None)) if directory.opaque else (
            False, CommandEffect(context_at(invocation.context, directory.cwd))
        )
    if invocation.executable in {".", "source"}:
        return True, CommandEffect(None)
    result = policy.command(invocation, context, depth)
    if result is not None:
        return result
    effect = after_external_command(
        invocation.executable, invocation.arguments, context,
    )
    return (True, CommandEffect(None)) if effect is None else (False, effect)


def _test_effect(arguments: list[str], context: ExecutionContext) -> CommandEffect:
    if len(arguments) != 2 or arguments[0] != "-e":
        return CommandEffect(context, context)
    path = Path(arguments[1])
    candidate = path if path.is_absolute() else Path(context.cwd) / path
    return CommandEffect(context) if candidate.exists() else CommandEffect(None, context)
