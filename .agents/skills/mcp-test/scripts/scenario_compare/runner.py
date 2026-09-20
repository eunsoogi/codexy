"""Run one unchanged scenario against two selected executor identities."""

from __future__ import annotations

from typing import Mapping

from scenario_flow import Scenario

from .diff import compare_results, normalizers as build_normalizers
from .types import (
    ComparisonValidationError,
    NormalizedField,
    Normalizer,
    ScenarioComparison,
    ScenarioExecutor,
    ScenarioRun,
)


def compare_scenario(
    scenario: Scenario,
    baseline: ScenarioExecutor,
    candidate: ScenarioExecutor,
    *,
    scenario_id: str = "scenario",
    normalizers: Mapping[NormalizedField | str, Normalizer] | None = None,
) -> ScenarioComparison:
    """Execute the exact scenario once per selected identity and compare it.

    The caller owns target selection and supplies one explicit normalizer per
    allowed ``(step, stored-field)`` pair. Timing and raw process output are
    intentionally excluded because this contract does not claim production
    determinism.
    """

    if not isinstance(scenario, Scenario):
        raise ComparisonValidationError("scenario must be a Scenario")
    if not isinstance(baseline, ScenarioExecutor):
        raise ComparisonValidationError("baseline must be a ScenarioExecutor")
    if not isinstance(candidate, ScenarioExecutor):
        raise ComparisonValidationError("candidate must be a ScenarioExecutor")
    if not isinstance(scenario_id, str) or not scenario_id:
        raise ComparisonValidationError("scenario_id must be non-empty")
    selected = build_normalizers(normalizers)
    baseline_result = baseline.execute(scenario)
    candidate_result = candidate.execute(scenario)
    differences = compare_results(
        scenario_id, baseline_result, candidate_result, selected
    )
    return ScenarioComparison(
        scenario_id,
        ScenarioRun(baseline.identity, baseline_result),
        ScenarioRun(candidate.identity, candidate_result),
        tuple(differences),
        frozenset(selected),
    )


compare = compare_scenario
