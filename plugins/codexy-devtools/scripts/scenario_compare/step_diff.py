"""Comparison of one named scenario step."""

from __future__ import annotations

from typing import Mapping

from scenario_flow import StepResult

from .diff import add_difference, compare_error, normalized_value
from .types import DifferenceKind, NormalizedField, Normalizer, ScenarioDifference


def compare_step(
    differences: list[ScenarioDifference],
    scenario: str,
    name: str,
    baseline: StepResult,
    candidate: StepResult,
    normalizers: Mapping[NormalizedField, Normalizer],
) -> None:
    if baseline.invoked != candidate.invoked:
        add_difference(
            differences,
            scenario,
            DifferenceKind.STEP_LINKAGE,
            name,
            "invoked",
            baseline.invoked,
            candidate.invoked,
            "step invocation changed",
        )
    if (baseline.execution is None) != (candidate.execution is None):
        add_difference(
            differences,
            scenario,
            DifferenceKind.SHAPE,
            name,
            "execution",
            baseline.execution is not None,
            candidate.execution is not None,
            "step execution result presence changed",
        )
    if (baseline.failure is None) != (candidate.failure is None):
        add_difference(
            differences,
            scenario,
            DifferenceKind.ERROR,
            name,
            "failure",
            baseline.failure is not None,
            candidate.failure is not None,
            "step failure presence changed",
        )
    if baseline.failure is not None and candidate.failure is not None:
        for field in ("kind", "message"):
            left = getattr(baseline.failure, field)
            right = getattr(candidate.failure, field)
            if left != right:
                add_difference(
                    differences,
                    scenario,
                    DifferenceKind.ERROR,
                    name,
                    f"failure.{field}",
                    getattr(left, "value", left),
                    getattr(right, "value", right),
                    f"step failure {field} changed",
                )
        for field in ("path", "blocked_by"):
            left = getattr(baseline.failure, field)
            right = getattr(candidate.failure, field)
            if left != right:
                add_difference(
                    differences,
                    scenario,
                    DifferenceKind.STEP_LINKAGE,
                    name,
                    f"failure.{field}",
                    left,
                    right,
                    f"step linkage {field} changed",
                )
    if baseline.execution is None or candidate.execution is None:
        return
    left_execution = baseline.execution
    right_execution = candidate.execution
    for field in ("kind", "tool", "protocol_version", "transport", "returncode"):
        left = getattr(left_execution, field)
        right = getattr(right_execution, field)
        if left != right:
            add_difference(
                differences,
                scenario,
                DifferenceKind.ERROR
                if field in {"kind", "returncode"}
                else DifferenceKind.SHAPE,
                name,
                f"execution.{field}",
                getattr(left, "value", left),
                getattr(right, "value", right),
                f"execution {field} changed",
            )
    if left_execution.meets_expectation != right_execution.meets_expectation:
        add_difference(
            differences,
            scenario,
            DifferenceKind.ERROR,
            name,
            "execution.meets_expectation",
            left_execution.meets_expectation,
            right_execution.meets_expectation,
            "expectation outcome changed",
        )
    compare_error(
        differences, scenario, name, left_execution.error, right_execution.error
    )
    left_fields = set(left_execution.stored)
    right_fields = set(right_execution.stored)
    if left_fields != right_fields:
        add_difference(
            differences,
            scenario,
            DifferenceKind.SHAPE,
            name,
            "stored_fields",
            tuple(sorted(left_fields)),
            tuple(sorted(right_fields)),
            "selected stored fields changed",
        )
    for field in sorted(left_fields & right_fields):
        key = (name, field)
        left = normalized_value(left_execution.stored[field], normalizers.get(key))
        right = normalized_value(right_execution.stored[field], normalizers.get(key))
        if left != right:
            add_difference(
                differences,
                scenario,
                DifferenceKind.VALUE,
                name,
                f"stored.{field}",
                left,
                right,
                "selected stored value changed",
            )
