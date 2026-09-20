"""Bounded field, error, shape, and step-linkage comparison helpers."""

from __future__ import annotations

import copy
from typing import Any, Mapping

from scenario_flow import ScenarioResult

from .types import (
    MISSING_FIELD,
    ComparisonValidationError,
    DifferenceKind,
    NormalizedField,
    Normalizer,
    ScenarioDifference,
)


def normalizers(
    values: Mapping[NormalizedField | str, Normalizer] | None,
) -> dict[NormalizedField, Normalizer]:
    """Validate the explicit step/field normalization policy."""

    normalized: dict[NormalizedField, Normalizer] = {}
    for raw_key, normalizer in (values or {}).items():
        if isinstance(raw_key, tuple) and len(raw_key) == 2:
            step, field = raw_key
        elif isinstance(raw_key, str) and "." in raw_key:
            step, field = raw_key.split(".", 1)
        else:
            raise ComparisonValidationError(
                "normalizer keys must be (step, field) tuples or step.field strings"
            )
        if (
            not isinstance(step, str)
            or not step
            or not isinstance(field, str)
            or not field
        ):
            raise ComparisonValidationError(
                "normalizer step and field must be non-empty"
            )
        if not callable(normalizer):
            raise ComparisonValidationError("normalizer must be callable")
        normalized[(step, field)] = normalizer
    return normalized


def normalized_value(value: Any, normalizer: Normalizer | None) -> Any:
    """Apply only a selected normalizer; fail closed on policy errors."""

    if value is MISSING_FIELD or normalizer is None:
        return copy.deepcopy(value)
    try:
        return copy.deepcopy(normalizer(value))
    except Exception as error:  # noqa: BLE001 - invalid policy must fail closed
        raise ComparisonValidationError("field normalizer failed") from error


def add_difference(
    differences: list[ScenarioDifference],
    scenario: str,
    kind: DifferenceKind,
    step: str | None,
    field: str,
    baseline: Any,
    candidate: Any,
    message: str,
) -> None:
    differences.append(
        ScenarioDifference(kind, scenario, step, field, baseline, candidate, message)
    )


def compare_error(
    differences: list[ScenarioDifference],
    scenario: str,
    step: str,
    baseline: Any,
    candidate: Any,
) -> None:
    if (baseline is None) != (candidate is None):
        add_difference(
            differences,
            scenario,
            DifferenceKind.ERROR,
            step,
            "execution.error",
            baseline is not None,
            candidate is not None,
            "error presence changed",
        )
        return
    if baseline is None:
        return
    for field in ("kind", "code", "message"):
        left = getattr(baseline, field)
        right = getattr(candidate, field)
        if left != right:
            add_difference(
                differences,
                scenario,
                DifferenceKind.ERROR,
                step,
                f"execution.error.{field}",
                getattr(left, "value", left),
                getattr(right, "value", right),
                f"execution error {field} changed",
            )


def compare_results(
    scenario: str,
    baseline: ScenarioResult,
    candidate: ScenarioResult,
    selected: Mapping[NormalizedField, Normalizer],
) -> list[ScenarioDifference]:
    """Compare observable results while ignoring timing and raw output."""

    from .step_diff import compare_step

    differences: list[ScenarioDifference] = []
    baseline_names = tuple(step.name for step in baseline.steps)
    candidate_names = tuple(step.name for step in candidate.steps)
    if baseline_names != candidate_names:
        add_difference(
            differences,
            scenario,
            DifferenceKind.SHAPE,
            None,
            "steps",
            baseline_names,
            candidate_names,
            "scenario step shape changed",
        )
    if baseline.failed_step != candidate.failed_step:
        add_difference(
            differences,
            scenario,
            DifferenceKind.STEP_LINKAGE,
            None,
            "failed_step",
            baseline.failed_step,
            candidate.failed_step,
            "failed step changed",
        )
    left_steps = {step.name: step for step in baseline.steps}
    right_steps = {step.name: step for step in candidate.steps}
    ordered_names = list(baseline_names) + [
        name for name in candidate_names if name not in left_steps
    ]
    for name in ordered_names:
        left = left_steps.get(name)
        right = right_steps.get(name)
        if left is None or right is None:
            add_difference(
                differences,
                scenario,
                DifferenceKind.SHAPE,
                name,
                "step",
                "present" if left else MISSING_FIELD,
                "present" if right else MISSING_FIELD,
                "step is missing from one run",
            )
            continue
        compare_step(differences, scenario, name, left, right, selected)
    return differences
