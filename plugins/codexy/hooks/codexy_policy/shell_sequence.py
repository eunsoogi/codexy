"""Evaluate supported shell connector paths without discarding alias outcomes."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import replace

from .execution_context import CommandEffect, ExecutionContext
from .shell_groups import Command, Group, Sequence

Segment = Callable[[list[str], ExecutionContext, int], tuple[bool, CommandEffect]]


def evaluate(sequence: Sequence, context: ExecutionContext, depth: int, segment: Segment) -> tuple[bool, ExecutionContext]:
    """Run ``&&`` success paths and join every path that reaches ``;``."""
    contexts, index = (context,), 0
    while index < len(sequence.steps):
        chain, separator = [], ""
        while index < len(sequence.steps):
            step = sequence.steps[index]
            chain.append(step)
            index += 1
            if step.following != "&&":
                separator = step.following
                break
        paths: list[ExecutionContext] = []
        for start in contexts:
            active, stopped = (start,), []
            for step in chain:
                next_active: list[ExecutionContext] = []
                for current in active:
                    denied, effect = _node(step.node, current, depth, segment)
                    if denied:
                        return True, context
                    if effect.failure is not None:
                        stopped.append(effect.failure)
                    if effect.success is not None:
                        next_active.append(effect.success)
                active = next_active
            paths.extend(stopped + active)
        contexts = tuple(_unique(paths))
        if separator == "":
            return False, _join(contexts, context)
        if separator != ";":
            return True, context
    return False, _join(contexts, context)


def _node(node: Command | Group, context: ExecutionContext, depth: int, segment: Segment) -> tuple[bool, CommandEffect]:
    if isinstance(node, Command):
        return segment(list(node.tokens), context, depth)
    denied, nested = evaluate(node.body, context, depth + 1, segment)
    if denied:
        return True, CommandEffect(None)
    return False, CommandEffect(context if node.kind == "subshell" else nested)


def _unique(contexts: list[ExecutionContext]) -> list[ExecutionContext]:
    return list(dict.fromkeys(contexts))


def _join(contexts: tuple[ExecutionContext, ...], fallback: ExecutionContext) -> ExecutionContext:
    if not contexts:
        return fallback
    aliases: dict[str, str] = {}
    for context in contexts:
        aliases.update(context.executable_aliases)
    return replace(contexts[0], executable_aliases=tuple(aliases.items()))
