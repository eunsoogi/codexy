"""Real two-server coverage for bounded MCP scenario comparison."""

from __future__ import annotations

import os
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path


REPOSITORY = Path(__file__).resolve().parents[3]
SCENARIO_SCRIPTS = REPOSITORY / ".agents/skills/mcp-test/scripts"
if str(SCENARIO_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCENARIO_SCRIPTS))

from scenario_compare import (  # noqa: E402
    ComparisonStatus,
    DifferenceKind,
    ScenarioExecutor,
    compare_scenario,
)
from scenario_core import ExpectedResult, SingleCall  # noqa: E402
from scenario_flow import (  # noqa: E402
    DataReference,
    Scenario,
    ScenarioResult,
    ScenarioStep,
    run_scenario,
)


FIXTURE_SOURCE = r"""
import json, sys
PROTOCOL = "2024-11-05"; step = sys.argv[sys.argv.index("--step") + 1]
variant = sys.argv[sys.argv.index("--variant") + 1]

def send(identifier, result=None, error=None):
    value = {"jsonrpc": "2.0", "id": identifier}
    value["error" if error is not None else "result"] = error or result
    print(json.dumps(value), flush=True)

for line in sys.stdin:
    request = json.loads(line)
    identifier = request.get("id")
    method = request.get("method")
    if identifier is None:
        continue
    if method == "initialize":
        send(identifier, {"protocolVersion": PROTOCOL, "serverInfo": {"name": "compare-fixture"}})
    elif method == "notifications/initialized":
        continue
    elif method == "tools/list":
        send(identifier, {"tools": [{"name": "read_value"}]})
    elif method == "tools/call":
        arguments = request.get("params", {}).get("arguments", {})
        if step == "search" and variant in {"baseline-failure", "candidate-error"}:
            code = -32001 if variant == "baseline-failure" else -32002
            send(identifier, error={"code": code, "message": "fixture-error"})
        elif step == "search":
            data = {"id": "item-42", "timestamp": "same" if variant in {"baseline", "candidate"} else variant, "public": "same", "secret": "secret-value"}
            if variant == "candidate-bad-id":
                data["id"] = "wrong-id"
            if variant == "candidate-missing":
                del data["timestamp"]
            send(identifier, {"content": [{"type": "text", "text": "ok"}], "data": data})
        else:
            send(identifier, {"content": [{"type": "text", "text": "ok"}], "data": {"matched_id": arguments.get("id"), "public": "same"}})
        break
"""


@unittest.skipUnless(os.name == "posix", "POSIX-only scenario execution")
class ScenarioComparisonTests(unittest.TestCase):
    def _fixture(self, root: Path) -> Path:
        path = root / "fixture_server.py"
        path.write_text(FIXTURE_SOURCE, encoding="utf-8")
        return path

    def _scenario(self, root: Path, fixture: Path, *, chain: bool = False) -> Scenario:
        def call(
            step: str, stored: dict[str, str], expected=ExpectedResult()
        ) -> SingleCall:
            arguments = {"id": "placeholder"} if step == "detail" else {}
            return SingleCall(
                argv=(
                    sys.executable,
                    "-u",
                    str(fixture),
                    "--step",
                    step,
                    "--variant",
                    "placeholder",
                ),
                cwd=root,
                environment={"SCENARIO_MARKER": "compare-test"},
                tool="read_value",
                allowed_tools=frozenset({"read_value"}),
                arguments=arguments,
                stored_fields=stored,
                expected=expected,
                timeout_seconds=2.0,
            )

        search = ScenarioStep(
            "search",
            call(
                "search",
                {
                    "id": "/result/data/id",
                    "timestamp": "/result/data/timestamp",
                    "public": "/result/data/public",
                },
            ),
        )
        if not chain:
            return Scenario(steps=(search,))
        detail = ScenarioStep(
            "detail",
            call(
                "detail",
                {"matched_id": "/result/data/matched_id"},
                ExpectedResult(fields={"/result/data/matched_id": "item-42"}),
            ),
            references={"/id": DataReference("search", "/id", str)},
        )
        return Scenario(steps=(search, detail))

    def _executor(self, identity: str) -> ScenarioExecutor:
        def run(scenario: Scenario) -> ScenarioResult:
            steps = tuple(
                replace(
                    step,
                    call=replace(step.call, argv=(*step.call.argv[:-1], identity)),
                )
                for step in scenario.steps
            )
            return run_scenario(replace(scenario, steps=steps))

        return ScenarioExecutor(identity, run)

    def test_equivalent_success_has_zero_exit(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            comparison = compare_scenario(
                self._scenario(root, self._fixture(root)),
                self._executor("baseline"),
                self._executor("candidate"),
                scenario_id="equivalent-success",
            )
            self.assertEqual(comparison.status, ComparisonStatus.MATCH)
            self.assertEqual(comparison.exit_code, 0)

    def test_timestamp_difference_is_only_normalized_when_declared(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scenario = self._scenario(root, self._fixture(root))
            raw = compare_scenario(
                scenario,
                self._executor("baseline"),
                self._executor("candidate-timestamp"),
            )
            self.assertEqual(raw.exit_code, 1)
            normalized = compare_scenario(
                scenario,
                self._executor("baseline"),
                self._executor("candidate-timestamp"),
                normalizers={("search", "timestamp"): lambda value: "timestamp"},
            )
            self.assertTrue(normalized.ok)
            self.assertEqual(normalized.exit_code, 0)

    def test_missing_field_is_reported_as_shape_difference(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            comparison = compare_scenario(
                self._scenario(root, self._fixture(root)),
                self._executor("baseline"),
                self._executor("candidate-missing"),
            )
            self.assertEqual(comparison.exit_code, 1)
            self.assertTrue(
                any(d.kind is DifferenceKind.SHAPE for d in comparison.differences)
            )

    def test_changed_error_behavior_is_visible_and_fails_ci(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            comparison = compare_scenario(
                self._scenario(root, self._fixture(root)),
                self._executor("baseline"),
                self._executor("candidate-error"),
            )
            self.assertEqual(comparison.status, ComparisonStatus.INCOMPARABLE)
            self.assertEqual(comparison.exit_code, 2)
            self.assertTrue(
                any(d.kind is DifferenceKind.ERROR for d in comparison.differences)
            )

    def test_bad_id_chaining_is_linked_to_the_dependent_step(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            comparison = compare_scenario(
                self._scenario(root, self._fixture(root), chain=True),
                self._executor("baseline"),
                self._executor("candidate-bad-id"),
                scenario_id="bad-id-chain",
            )
            self.assertEqual(comparison.candidate.result.failed_step, "detail")
            self.assertTrue(any(d.step == "detail" for d in comparison.differences))

    def test_matching_failures_never_pass_and_baseline_stays_visible(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            comparison = compare_scenario(
                self._scenario(root, self._fixture(root)),
                self._executor("baseline-failure"),
                self._executor("baseline-failure"),
                scenario_id="matching-failure",
            )
            self.assertEqual(comparison.status, ComparisonStatus.INCOMPARABLE)
            self.assertEqual(comparison.exit_code, 2)
            self.assertEqual(comparison.baseline.result.failed_step, "search")

    def test_only_selected_fields_are_persisted_in_comparison_runs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scenario = self._scenario(root, self._fixture(root))
            scenario = replace(
                scenario,
                steps=(
                    replace(
                        scenario.steps[0],
                        call=replace(
                            scenario.steps[0].call,
                            stored_fields={"public": "/result/data/public"},
                        ),
                    ),
                ),
            )
            comparison = compare_scenario(
                scenario,
                self._executor("baseline"),
                self._executor("candidate"),
                scenario_id="selected-fields",
            )
            stored = comparison.baseline.result.steps[0].execution.stored
            self.assertEqual(dict(stored), {"public": "same"})
            self.assertNotIn("secret-value", repr(comparison))
