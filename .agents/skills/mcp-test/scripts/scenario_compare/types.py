"""Public contracts for comparing two bounded MCP scenario executions."""

from __future__ import annotations

import copy
from dataclasses import dataclass
from enum import Enum
from typing import Any, Callable, Mapping

from scenario_flow import Scenario, ScenarioResult


EXIT_SUCCESS = 0
EXIT_DIFFERENCE = 1
EXIT_INCOMPARABLE = 2
MISSING_FIELD = "<missing>"

Normalizer = Callable[[Any], Any]
NormalizedField = tuple[str, str]


class ComparisonValidationError(ValueError):
    """Raised when comparison inputs leave the declared comparison boundary."""


class DifferenceKind(str, Enum):
    """Categories that keep comparison output actionable and bounded."""

    SHAPE = "shape"
    VALUE = "value"
    ERROR = "error"
    STEP_LINKAGE = "step-linkage"


class ComparisonStatus(str, Enum):
    """The CI-level outcome of one baseline/candidate comparison."""

    MATCH = "match"
    DIFFERENT = "different"
    INCOMPARABLE = "incomparable"


@dataclass(frozen=True)
class ScenarioExecutor:
    """An executor-selected version identity and its scenario runner."""

    identity: str
    run: Callable[[Scenario], ScenarioResult]

    def __post_init__(self) -> None:
        if not isinstance(self.identity, str) or not self.identity:
            raise ComparisonValidationError("executor identity must be non-empty")
        if not callable(self.run):
            raise ComparisonValidationError("executor run must be callable")

    def execute(self, scenario: Scenario) -> ScenarioResult:
        """Run the supplied scenario without changing its declared inputs."""

        result = self.run(scenario)
        if not isinstance(result, ScenarioResult):
            raise ComparisonValidationError("executor must return a ScenarioResult")
        return result


@dataclass(frozen=True)
class ScenarioRun:
    """One immutable version-linked result retained by a comparison report."""

    identity: str
    result: ScenarioResult

    def __post_init__(self) -> None:
        if not isinstance(self.identity, str) or not self.identity:
            raise ComparisonValidationError("run identity must be non-empty")
        if not isinstance(self.result, ScenarioResult):
            raise ComparisonValidationError("run result must be a ScenarioResult")


@dataclass(frozen=True)
class ScenarioDifference:
    """One scenario- and step-linked observable difference."""

    kind: DifferenceKind
    scenario: str
    step: str | None
    field: str
    baseline_value: Any
    candidate_value: Any
    message: str

    def __post_init__(self) -> None:
        if not isinstance(self.kind, DifferenceKind):
            raise ComparisonValidationError("difference kind must be a DifferenceKind")
        if not isinstance(self.scenario, str) or not self.scenario:
            raise ComparisonValidationError("difference scenario must be non-empty")
        if self.step is not None and (not isinstance(self.step, str) or not self.step):
            raise ComparisonValidationError("difference step must be non-empty")
        if not isinstance(self.field, str) or not self.field:
            raise ComparisonValidationError("difference field must be non-empty")
        if not isinstance(self.message, str) or not self.message:
            raise ComparisonValidationError("difference message must be non-empty")
        object.__setattr__(self, "baseline_value", copy.deepcopy(self.baseline_value))
        object.__setattr__(self, "candidate_value", copy.deepcopy(self.candidate_value))

    @property
    def path(self) -> str:
        """Return a stable report path for this difference."""

        return f"{self.step}.{self.field}" if self.step else self.field

    def to_dict(self) -> dict[str, Any]:
        """Return a JSON-safe difference record for a future CLI."""

        return {
            "kind": self.kind.value,
            "scenario": self.scenario,
            "step": self.step,
            "field": self.field,
            "path": self.path,
            "baseline": copy.deepcopy(self.baseline_value),
            "candidate": copy.deepcopy(self.candidate_value),
            "message": self.message,
        }


@dataclass(frozen=True)
class ScenarioComparison:
    """Complete baseline/candidate report and its CI exit-code contract."""

    scenario: str
    baseline: ScenarioRun
    candidate: ScenarioRun
    differences: tuple[ScenarioDifference, ...]
    normalized_fields: frozenset[NormalizedField]

    def __post_init__(self) -> None:
        if not isinstance(self.scenario, str) or not self.scenario:
            raise ComparisonValidationError("scenario identity must be non-empty")
        object.__setattr__(self, "differences", tuple(self.differences))
        object.__setattr__(self, "normalized_fields", frozenset(self.normalized_fields))

    @property
    def status(self) -> ComparisonStatus:
        if not self.baseline.result.ok or not self.candidate.result.ok:
            return ComparisonStatus.INCOMPARABLE
        if self.differences:
            return ComparisonStatus.DIFFERENT
        return ComparisonStatus.MATCH

    @property
    def ok(self) -> bool:
        """Return true only when both runs succeeded and matched."""

        return self.status is ComparisonStatus.MATCH

    @property
    def exit_code(self) -> int:
        """Return the stable CI contract: 0 match, 1 difference, 2 failure."""

        return {
            ComparisonStatus.MATCH: EXIT_SUCCESS,
            ComparisonStatus.DIFFERENT: EXIT_DIFFERENCE,
            ComparisonStatus.INCOMPARABLE: EXIT_INCOMPARABLE,
        }[self.status]

    def to_dict(self) -> dict[str, Any]:
        """Return bounded report metadata without raw process output."""

        def summary(run: ScenarioRun) -> dict[str, Any]:
            return {
                "identity": run.identity,
                "status": run.result.status,
                "ok": run.result.ok,
                "failed_step": run.result.failed_step,
            }

        return {
            "scenario": self.scenario,
            "baseline": summary(self.baseline),
            "candidate": summary(self.candidate),
            "status": self.status.value,
            "exit_code": self.exit_code,
            "normalized_fields": [
                {"step": step, "field": field}
                for step, field in sorted(self.normalized_fields)
            ],
            "differences": [difference.to_dict() for difference in self.differences],
        }


# Short aliases keep the public surface readable for callers that prefer the
# generic terms while retaining the explicit names used in reports.
Executor = ScenarioExecutor
Difference = ScenarioDifference
ComparisonResult = ScenarioComparison
