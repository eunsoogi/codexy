"""Version-linked comparison for bounded MCP scenarios."""

from .runner import compare, compare_scenario
from .types import (
    EXIT_DIFFERENCE,
    EXIT_INCOMPARABLE,
    EXIT_SUCCESS,
    ComparisonResult,
    ComparisonStatus,
    ComparisonValidationError,
    Difference,
    DifferenceKind,
    Executor,
    ScenarioComparison,
    ScenarioDifference,
    ScenarioExecutor,
    ScenarioRun,
)

# Keep low-level comparison helpers private; callers use the types and entry points.
__all__ = [
    "ComparisonResult",
    "ComparisonStatus",
    "ComparisonValidationError",
    "Difference",
    "DifferenceKind",
    "EXIT_DIFFERENCE",
    "EXIT_INCOMPARABLE",
    "EXIT_SUCCESS",
    "Executor",
    "ScenarioComparison",
    "ScenarioDifference",
    "ScenarioExecutor",
    "ScenarioRun",
    "compare",
    "compare_scenario",
]
