"""Evaluate supported shell connector paths without discarding alias outcomes."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import replace

from .execution_context import CommandEffect, ExecutionContext
from .shell_groups import Command, Group, Sequence

Segment = Callable[[list[str], ExecutionContext, int], tuple[bool, CommandEffect]]


def evaluate(
    sequence: Sequence, context: ExecutionContext, depth: int, segment: Segment
) -> tuple[bool, ExecutionContext]:
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
            return True, context
        contexts = tuple(_unique(success + failure))
        if separator == "":
            return False, _join(contexts, context)
        if separator not in {";", "|", "&"}:
            return True, context
    return False, _join(contexts, context)


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
    node: Command | Group, context: ExecutionContext, depth: int, segment: Segment
) -> tuple[bool, CommandEffect]:
    if isinstance(node, Command):
        return segment(list(node.tokens), context, depth)
    denied, nested = evaluate(node.body, context, depth + 1, segment)
    if denied:
        return True, CommandEffect(None)
    return False, CommandEffect(context if node.kind == "subshell" else nested)


def _unique(contexts: list[ExecutionContext]) -> list[ExecutionContext]:
    return list(dict.fromkeys(contexts))


def _join(
    contexts: tuple[ExecutionContext, ...], fallback: ExecutionContext
) -> ExecutionContext:
    if not contexts:
        return fallback
    aliases: dict[str, str] = {}
    for context in contexts:
        aliases.update(context.executable_aliases)
    return replace(contexts[0], executable_aliases=tuple(aliases.items()))
