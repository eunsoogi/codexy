"""Contracts for ordered, fail-fast MCP scenario execution."""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from enum import Enum
from types import MappingProxyType
from typing import Mapping

from scenario_core import ExecutionResult, SingleCall


def _pointer(path: str, label: str) -> str:
    if not isinstance(path, str) or not path.startswith("/") or path == "/":
        raise ValueError(f"{label} must be a non-root JSON Pointer")
    return path


def _type_spec(value: object) -> bool:
    return isinstance(value, type) or (
        isinstance(value, tuple)
        and bool(value)
        and all(isinstance(item, type) for item in value)
    )


class FailureKind(str, Enum):
    """Failure locations surfaced by the ordered runner."""

    REFERENCE_ERROR = "reference-error"
    SCHEMA_MISMATCH = "schema-mismatch"
    VALUE_MISMATCH = "value-mismatch"
    EXPECTED_ERROR_MISMATCH = "expected-error-mismatch"
    EXECUTION_ERROR = "execution-error"
    PREDECESSOR_FAILED = "predecessor-failed"
    DEADLINE_EXCEEDED = "deadline-exceeded"


@dataclass(frozen=True)
class DataReference:
    """A typed JSON-pointer lookup into an earlier step's stored fields."""

    step: str
    path: str
    expected_type: type | tuple[type, ...] | None = None

    def __post_init__(self) -> None:
        if not isinstance(self.step, str) or not self.step:
            raise ValueError("reference step must be a non-empty string")
        _pointer(self.path, "reference path")
        if self.expected_type is not None and not _type_spec(self.expected_type):
            raise ValueError(
                "reference expected_type must be a type or non-empty tuple"
            )


@dataclass(frozen=True)
class ScenarioStep:
    """One explicit call and the argument locations filled from prior data."""

    name: str
    call: SingleCall
    references: Mapping[str, DataReference] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.name, str) or not self.name:
            raise ValueError("step name must be a non-empty string")
        if not isinstance(self.call, SingleCall):
            raise ValueError("step call must be a SingleCall")
        if not isinstance(self.references, Mapping):
            raise ValueError("step references must be a mapping")
        references = {}
        for target, reference in self.references.items():
            _pointer(target, "reference target")
            if not isinstance(reference, DataReference):
                raise ValueError("step references must contain DataReference values")
            references[target] = reference
        object.__setattr__(self, "references", MappingProxyType(references))


@dataclass(frozen=True)
class Scenario:
    """An ordered sequence with one wall-clock deadline and no branching."""

    steps: tuple[ScenarioStep, ...]
    deadline_seconds: float = 30.0

    def __post_init__(self) -> None:
        steps = tuple(self.steps)
        if not steps or any(not isinstance(step, ScenarioStep) for step in steps):
            raise ValueError(
                "scenario steps must be a non-empty sequence of ScenarioStep"
            )
        names = [step.name for step in steps]
        if len(names) != len(set(names)):
            raise ValueError("scenario step names must be unique")
        seen = set()
        for step in steps:
            for reference in step.references.values():
                if reference.step not in seen:
                    raise ValueError("references must target an earlier step")
            seen.add(step.name)
        if (
            isinstance(self.deadline_seconds, bool)
            or not isinstance(self.deadline_seconds, (int, float))
            or not math.isfinite(self.deadline_seconds)
            or self.deadline_seconds <= 0
        ):
            raise ValueError("deadline_seconds must be a finite positive number")
        object.__setattr__(self, "steps", steps)


@dataclass(frozen=True)
class StepFailure:
    kind: FailureKind
    message: str
    path: str | None = None
    blocked_by: str | None = None


@dataclass(frozen=True)
class StepResult:
    name: str
    invoked: bool
    execution: ExecutionResult | None
    failure: StepFailure | None = None

    @property
    def ok(self) -> bool:
        return self.failure is None

    @property
    def status(self) -> str:
        return "success" if self.failure is None else self.failure.kind.value


@dataclass(frozen=True)
class ScenarioResult:
    steps: tuple[StepResult, ...]
    failed_step: str | None
    elapsed_seconds: float

    @property
    def ok(self) -> bool:
        return self.failed_step is None and all(step.ok for step in self.steps)

    @property
    def status(self) -> str:
        if self.ok:
            return "success"
        return next(step.status for step in self.steps if not step.ok)
