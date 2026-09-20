"""Real subprocess coverage for ordered MCP scenario execution."""

from __future__ import annotations

import json
import sys
import tempfile
import time
import unittest
from pathlib import Path


REPOSITORY = Path(__file__).resolve().parents[3]
SCENARIO_SCRIPTS = REPOSITORY / "plugins/codexy-devtools/scripts"
FIXTURE = Path(__file__).with_name("test_mcp_scenario_flow") / "fixture_server.py"
if str(SCENARIO_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCENARIO_SCRIPTS))

from scenario_core import ExpectedResult, ResultKind, SingleCall  # noqa: E402
from scenario_flow import (  # noqa: E402
    DataReference,
    FailureKind,
    Scenario,
    ScenarioStep,
    run_scenario,
)


class ScenarioFlowTests(unittest.TestCase):
    def _call(self, root: Path, mode: str, record: Path, **kwargs) -> SingleCall:
        arguments = kwargs.pop("arguments", {})
        expected = kwargs.pop("expected", ExpectedResult())
        stored_fields = kwargs.pop("stored_fields", {})
        value = kwargs.pop("value", None)
        argv = [sys.executable, "-u", str(FIXTURE), "--mode", mode, "--record", str(record)]
        if value is not None:
            argv.extend(("--value", value))
        return SingleCall(
            argv=tuple(argv),
            cwd=root,
            environment={"SCENARIO_MARKER": "flow-test"},
            tool="read_value",
            allowed_tools=frozenset({"read_value"}),
            arguments=arguments,
            stored_fields=stored_fields,
            expected=expected,
            timeout_seconds=kwargs.pop("timeout_seconds", 2.0),
        )

    def _search_detail(self, root: Path, mode: str, detail_record: Path, **kwargs) -> Scenario:
        search = self._call(
            root,
            mode,
            root / "search.json",
            stored_fields={"id": "/result/data/id"},
            expected=kwargs.pop("search_expected", ExpectedResult()),
            value=kwargs.pop("value", None),
            timeout_seconds=kwargs.pop("search_timeout", 2.0),
        )
        detail = self._call(
            root,
            "detail",
            detail_record,
            arguments={"id": "placeholder"},
            stored_fields={"matched_id": "/result/data/matched_id"},
            expected=kwargs.pop(
                "detail_expected",
                ExpectedResult(fields={"/result/data/matched_id": "item-42"}),
            ),
        )
        return Scenario(
            steps=(
                ScenarioStep("search", search),
                ScenarioStep(
                    "detail",
                    detail,
                    references={
                        "/id": DataReference("search", "/id", kwargs.pop("ref_type", str))
                    },
                ),
            ),
            deadline_seconds=kwargs.pop("deadline_seconds", 30.0),
        )

    def test_search_to_detail_uses_returned_id_in_order(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            detail_record = root / "detail.json"
            scenario = self._search_detail(root, "search", detail_record)
            detail_call = scenario.steps[1].call
            result = run_scenario(scenario)
            self.assertTrue(result.ok)
            self.assertEqual(result.status, "success")
            self.assertEqual(json.loads(detail_record.read_text())["arguments"]["id"], "item-42")
            self.assertEqual(result.steps[1].execution.stored["matched_id"], "item-42")
            self.assertEqual(detail_call.argv, scenario.steps[1].call.argv)
            self.assertEqual(detail_call.cwd, scenario.steps[1].call.cwd)
            self.assertEqual(detail_call.environment, scenario.steps[1].call.environment)
            self.assertEqual(detail_call.tool, scenario.steps[1].call.tool)
            self.assertEqual(detail_call.allowed_tools, scenario.steps[1].call.allowed_tools)

    def test_reference_failures_are_reported_at_dependent_step(self) -> None:
        for mode in ("missing-id", "wrong-type"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                detail_record = root / "detail.json"
                result = run_scenario(self._search_detail(root, mode, detail_record))
                self.assertEqual(result.failed_step, "detail")
                self.assertEqual(result.steps[1].failure.kind, FailureKind.REFERENCE_ERROR)
                self.assertFalse(result.steps[1].invoked)
                self.assertFalse(detail_record.exists())

    def test_failed_predecessor_suppresses_dependent_invocation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            detail_record = root / "detail.json"
            result = run_scenario(
                self._search_detail(
                    root,
                    "wrong-value",
                    detail_record,
                    search_expected=ExpectedResult(fields={"/result/data/id": "item-42"}),
                )
            )
            self.assertEqual(result.failed_step, "search")
            self.assertEqual(result.steps[0].failure.kind, FailureKind.VALUE_MISMATCH)
            self.assertEqual(result.steps[1].failure.kind, FailureKind.PREDECESSOR_FAILED)
            self.assertEqual(result.status, FailureKind.VALUE_MISMATCH.value)
            self.assertFalse(result.steps[1].invoked)
            self.assertFalse(detail_record.exists())

    def test_schema_and_expected_error_assertions_are_step_results(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            malformed = run_scenario(
                Scenario(
                    steps=(
                        ScenarioStep(
                            "malformed", self._call(root, "malformed", root / "malformed.json")
                        ),
                    )
                )
            )
            self.assertEqual(malformed.steps[0].failure.kind, FailureKind.SCHEMA_MISMATCH)
            expected_error = run_scenario(
                Scenario(
                    steps=(
                        ScenarioStep(
                            "expected-error",
                            self._call(
                                root,
                                "expected-error",
                                root / "expected-error.json",
                                expected=ExpectedResult(
                                    kind=ResultKind.JSON_RPC_ERROR,
                                    fields={"/error/code": -32020},
                                ),
                            ),
                        ),
                    )
                )
            )
            self.assertTrue(expected_error.ok)
            self.assertTrue(expected_error.steps[0].ok)

    def test_response_strings_are_inert_data(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root / "must-not-exist"
            malicious = f"$(touch {marker})"
            detail_record = root / "detail.json"
            result = run_scenario(
                self._search_detail(
                    root,
                    "malicious",
                    detail_record,
                    value=malicious,
                    detail_expected=ExpectedResult(
                        fields={"/result/data/matched_id": malicious}
                    ),
                )
            )
            self.assertTrue(result.ok)
            self.assertFalse(marker.exists())
            self.assertEqual(json.loads(detail_record.read_text())["arguments"]["id"], malicious)

    def test_whole_scenario_deadline_stops_later_steps(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            detail_record = root / "detail.json"
            started = time.monotonic()
            result = run_scenario(
                self._search_detail(
                    root,
                    "slow",
                    detail_record,
                    search_timeout=5.0,
                    deadline_seconds=0.25,
                )
            )
            self.assertLess(time.monotonic() - started, 2.0)
            self.assertEqual(result.failed_step, "search")
            self.assertEqual(result.steps[0].failure.kind, FailureKind.DEADLINE_EXCEEDED)
            self.assertEqual(result.steps[1].failure.kind, FailureKind.PREDECESSOR_FAILED)
            self.assertFalse(detail_record.exists())

    def test_references_must_target_an_earlier_step(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            call = self._call(root, "detail", root / "detail.json", arguments={"id": "x"})
            with self.assertRaises(ValueError):
                Scenario(
                    steps=(
                        ScenarioStep(
                            "detail",
                            call,
                            references={"/id": DataReference("search", "/id", str)},
                        ),
                    )
                )


if __name__ == "__main__":
    unittest.main()
