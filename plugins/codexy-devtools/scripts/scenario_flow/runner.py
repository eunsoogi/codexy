"""Ordered data-reference resolution around the bounded single-call runner."""

from __future__ import annotations

import copy
from dataclasses import replace
from time import monotonic
from typing import Any, Mapping

from scenario_core import ResultKind, run_single_call

from .types import (
    DataReference,
    FailureKind,
    Scenario,
    ScenarioResult,
    ScenarioStep,
    StepFailure,
    StepResult,
)


class _ReferenceProblem(Exception):
    def __init__(self, reference: DataReference, path: str, message: str) -> None:
        super().__init__(message)
        self.reference = reference
        self.path = path


def _tokens(path: str) -> list[str]:
    return [token.replace("~1", "/").replace("~0", "~") for token in path[1:].split("/")]


def _get(value: object, path: str) -> object:
    current = value
    for token in _tokens(path):
        if isinstance(current, Mapping) and token in current:
            current = current[token]
        elif isinstance(current, list):
            try:
                current = current[int(token)]
            except (ValueError, IndexError):
                raise KeyError(path) from None
        else:
            raise KeyError(path)
    return current


def _set(value: object, path: str, replacement: object) -> None:
    current = value
    tokens = _tokens(path)
    for token in tokens[:-1]:
        if isinstance(current, dict) and token in current:
            current = current[token]
        elif isinstance(current, list):
            try:
                current = current[int(token)]
            except (ValueError, IndexError):
                raise KeyError(path) from None
        else:
            raise KeyError(path)
    final = tokens[-1]
    if isinstance(current, dict) and final in current:
        current[final] = replacement
    elif isinstance(current, list):
        try:
            current[int(final)] = replacement
        except (ValueError, IndexError):
            raise KeyError(path) from None
    else:
        raise KeyError(path)


def _matches_type(value: object, expected: type | tuple[type, ...]) -> bool:
    if expected is int:
        return isinstance(value, int) and not isinstance(value, bool)
    if isinstance(expected, tuple):
        return any(_matches_type(value, item) for item in expected)
    return isinstance(value, expected)


def _resolve_call(step: ScenarioStep, completed: Mapping[str, StepResult]):
    arguments = copy.deepcopy(dict(step.call.arguments))
    for target, reference in step.references.items():
        source = completed.get(reference.step)
        if source is None or source.execution is None:
            raise _ReferenceProblem(reference, reference.path, "referenced step has no result")
        try:
            value = _get(source.execution.stored, reference.path)
        except KeyError:
            raise _ReferenceProblem(reference, reference.path, "referenced field is missing") from None
        if reference.expected_type is not None and not _matches_type(value, reference.expected_type):
            raise _ReferenceProblem(reference, target, "referenced value has the wrong type")
        try:
            _set(arguments, target, value)
        except KeyError:
            raise _ReferenceProblem(reference, target, "reference target is missing") from None
    return replace(step.call, arguments=arguments)


def _failure_for_execution(call, execution) -> StepFailure | None:
    expected = call.expected.kind
    if execution.kind is ResultKind.MALFORMED_RESULT and expected is not ResultKind.MALFORMED_RESULT:
        return StepFailure(FailureKind.SCHEMA_MISMATCH, "step returned a malformed result")
    if execution.kind is not expected:
        if expected is not ResultKind.SUCCESS:
            return StepFailure(FailureKind.EXPECTED_ERROR_MISMATCH, "step did not meet its expected error")
        return StepFailure(FailureKind.EXECUTION_ERROR, f"step returned {execution.kind.value}")
    if not execution.meets_expectation:
        if expected is not ResultKind.SUCCESS:
            return StepFailure(FailureKind.EXPECTED_ERROR_MISMATCH, "step did not meet its expected error")
        return StepFailure(FailureKind.VALUE_MISMATCH, "step returned an unexpected value")
    return None


def _blocked(step: ScenarioStep, predecessor: str) -> StepResult:
    return StepResult(
        name=step.name,
        invoked=False,
        execution=None,
        failure=StepFailure(
            FailureKind.PREDECESSOR_FAILED,
            "step was suppressed after a predecessor failed",
            blocked_by=predecessor,
        ),
    )


def run_scenario(scenario: Scenario, cancellation: Any = None) -> ScenarioResult:
    """Run steps in declaration order and stop at the first failed step."""

    started = monotonic()
    completed: dict[str, StepResult] = {}
    results: list[StepResult] = []
    failed_step = None
    for index, step in enumerate(scenario.steps):
        remaining = scenario.deadline_seconds - (monotonic() - started)
        if remaining <= 0:
            failed_step = step.name
            result = StepResult(
                step.name,
                False,
                None,
                StepFailure(FailureKind.DEADLINE_EXCEEDED, "scenario deadline exceeded"),
            )
        else:
            try:
                call = _resolve_call(step, completed)
            except _ReferenceProblem as problem:
                failed_step = step.name
                result = StepResult(
                    step.name,
                    False,
                    None,
                    StepFailure(FailureKind.REFERENCE_ERROR, str(problem), problem.path),
                )
            else:
                deadline_limited = call.timeout_seconds > remaining
                bounded_call = replace(call, timeout_seconds=min(call.timeout_seconds, remaining))
                execution = run_single_call(bounded_call, cancellation)
                failure = _failure_for_execution(bounded_call, execution)
                if execution.kind is ResultKind.TIMEOUT and deadline_limited:
                    failure = StepFailure(FailureKind.DEADLINE_EXCEEDED, "scenario deadline exceeded")
                if failure is not None:
                    failed_step = step.name
                result = StepResult(step.name, True, execution, failure)
        results.append(result)
        completed[step.name] = result
        if failed_step is not None:
            results.extend(_blocked(later, failed_step) for later in scenario.steps[index + 1 :])
            break
    elapsed = monotonic() - started
    return ScenarioResult(tuple(results), failed_step, elapsed)
