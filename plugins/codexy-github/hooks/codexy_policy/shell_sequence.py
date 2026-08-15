"""Evaluate supported shell connector paths without discarding alias outcomes."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import replace

from .execution_context import CommandEffect, ExecutionContext
from .shell_groups import Command, Conditional, Group, Sequence

Segment = Callable[[list[str], ExecutionContext, int], tuple[bool, CommandEffect]]


def evaluate(
    sequence: Sequence, context: ExecutionContext, depth: int, segment: Segment
) -> tuple[bool, CommandEffect]:
    """Evaluate supported connector lists without discarding result branches."""
    contexts, index = (context,), 0
    while index < len(sequence.steps):
        nodes, connectors, separator = [], [], ""
        while index < len(sequence.steps):
            step = sequence.steps[index]
            nodes.append(step.node)
            index += 1
            if step.following in {"&&", "||"}:
                connectors.append(step.following)
            else:
                separator = step.following
                break
        denied, success, failure = _list(nodes, connectors, contexts, depth, segment)
        if denied:
            return True, CommandEffect(None)
        contexts = tuple(_unique(success + failure))
        if separator == "":
            return False, _effect(success, failure)
        if separator not in {";", "|", "&"}:
            return True, CommandEffect(None)
    return False, CommandEffect(context)


def _list(
    nodes: list[Command | Group],
    connectors: list[str],
    contexts: tuple[ExecutionContext, ...],
    depth: int,
    segment: Segment,
) -> tuple[bool, list[ExecutionContext], list[ExecutionContext]]:
    denied, success, failure = _apply(nodes[0], contexts, depth, segment)
    if denied:
        return True, [], []
    for connector, node in zip(connectors, nodes[1:]):
        carried, active = (
            (failure, success) if connector == "&&" else (success, failure)
        )
        denied, next_success, next_failure = _apply(node, tuple(active), depth, segment)
        if denied:
            return True, [], []
        if connector == "&&":
            success, failure = next_success, carried + next_failure
        else:
            success, failure = carried + next_success, next_failure
    return False, success, failure


def _apply(
    node: Command | Group,
    contexts: tuple[ExecutionContext, ...],
    depth: int,
    segment: Segment,
) -> tuple[bool, list[ExecutionContext], list[ExecutionContext]]:
    success, failure = [], []
    for context in contexts:
        denied, effect = _node(node, context, depth, segment)
        if denied:
            return True, [], []
        if effect.success is not None:
            success.append(effect.success)
        if effect.failure is not None:
            failure.append(effect.failure)
    return False, success, failure


def _node(
    node: Command | Conditional | Group,
    context: ExecutionContext,
    depth: int,
    segment: Segment,
) -> tuple[bool, CommandEffect]:
    if isinstance(node, Command):
        return segment(list(node.tokens), context, depth)
    if isinstance(node, Conditional):
        return _conditional(node, context, depth, segment)
    denied, nested = evaluate(node.body, context, depth + 1, segment)
    if denied:
        return True, CommandEffect(None)
    if node.kind == "subshell":
        return False, CommandEffect(
            context if nested.success is not None else None,
            context if nested.failure is not None else None,
        )
    return False, nested


def _conditional(
    node: Conditional, context: ExecutionContext, depth: int, segment: Segment
) -> tuple[bool, CommandEffect]:
    denied, condition = evaluate(node.condition, context, depth + 1, segment)
    if denied:
        return True, CommandEffect(None)
    effects: list[CommandEffect] = []
    if condition.success is not None:
        denied, effect = evaluate(
            node.then_branch, condition.success, depth + 1, segment
        )
        if denied:
            return True, CommandEffect(None)
        effects.append(effect)
    if condition.failure is not None:
        if node.else_branch is None:
            effects.append(CommandEffect(None, condition.failure))
        else:
            denied, effect = evaluate(
                node.else_branch, condition.failure, depth + 1, segment
            )
            if denied:
                return True, CommandEffect(None)
            effects.append(effect)
    return False, CommandEffect(
        _join([effect.success for effect in effects if effect.success is not None]),
        _join([effect.failure for effect in effects if effect.failure is not None]),
    )


def _unique(contexts: list[ExecutionContext]) -> list[ExecutionContext]:
    return list(dict.fromkeys(contexts))


def _effect(
    success: list[ExecutionContext], failure: list[ExecutionContext]
) -> CommandEffect:
    return CommandEffect(_join(success), _join(failure))


def _join(contexts: list[ExecutionContext]) -> ExecutionContext | None:
    if not contexts:
        return None
    aliases: dict[str, str] = {}
    for context in contexts:
        aliases.update(context.executable_aliases)
    return replace(contexts[0], executable_aliases=tuple(aliases.items()))
