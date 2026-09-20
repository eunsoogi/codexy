"""Real subprocess coverage for the bounded single-call MCP scenario core."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path


REPOSITORY = Path(__file__).resolve().parents[3]
SCENARIO_SCRIPTS = REPOSITORY / "plugins/codexy-devtools/scripts"
FIXTURE = Path(__file__).with_name("mcp_scenario_fixtures") / "fixture_server.py"
if str(SCENARIO_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCENARIO_SCRIPTS))

from scenario_core import (  # noqa: E402
    CancellationToken,
    ExpectedResult,
    ResultKind,
    ScenarioValidationError,
    SingleCall,
    UnsupportedProtocolError,
    UnsupportedTransportError,
    run_single_call,
    supported_versions,
)


def _pid_active(pid: int) -> bool:
    if os.name == "nt":
        result = subprocess.run(
            ["tasklist", "/FI", f"PID eq {pid}", "/FO", "CSV", "/NH"],
            capture_output=True,
            text=True,
            check=False,
            timeout=5,
        )
        return any(
            row.split(",")[1].strip('"') == str(pid)
            for row in result.stdout.splitlines()
            if "," in row
        )
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


class SingleCallScenarioTests(unittest.TestCase):
    def _call(self, root: Path, mode: str, **kwargs) -> SingleCall:
        return SingleCall(
            argv=(
                sys.executable,
                "-u",
                str(FIXTURE),
                "--mode",
                mode,
                *kwargs.pop("argv_tail", ()),
            ),
            cwd=root,
            environment=kwargs.pop("environment", {}),
            tool="read_value",
            allowed_tools=frozenset({"read_value"}),
            arguments=kwargs.pop("arguments", {}),
            stored_fields=kwargs.pop(
                "stored_fields",
                {
                    "text": "/result/content/0/text",
                    "public": "/result/data/public",
                },
            ),
            expected=kwargs.pop("expected", ExpectedResult()),
            protocol_version=kwargs.pop("protocol_version", "2024-11-05"),
            transport=kwargs.pop("transport", "stdio-newline-v1"),
            timeout_seconds=kwargs.pop("timeout_seconds", 2.0),
            output_limit_bytes=kwargs.pop("output_limit_bytes", 64 * 1024),
        )

    def test_support_contract_is_explicit_and_rejects_unknown_variants(self) -> None:
        support = supported_versions()
        self.assertEqual(support["protocol_versions"], ["2024-11-05"])
        self.assertEqual(support["transports"], ["stdio-newline-v1"])
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(UnsupportedProtocolError):
                self._call(Path(directory), "success", protocol_version="2026-07-28")
            with self.assertRaises(UnsupportedTransportError):
                self._call(Path(directory), "success", transport="streamable-http")
            for timeout in (float("nan"), float("inf"), "not-a-number"):
                with self.assertRaises(ScenarioValidationError):
                    self._call(Path(directory), "success", timeout_seconds=timeout)

    def test_initialize_completes_before_follow_up_requests(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = run_single_call(self._call(Path(directory), "delayed-init"))
            self.assertEqual(result.kind, ResultKind.SUCCESS)

    def test_server_selected_protocol_must_be_supported(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = run_single_call(
                self._call(Path(directory), "unsupported-version", stored_fields={})
            )
            self.assertEqual(result.kind, ResultKind.UNSUPPORTED_PROTOCOL)

    def test_success_uses_explicit_invocation_and_selected_persistence(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            record = root / "record.json"
            call = self._call(
                root,
                "success",
                environment={"SCENARIO_MARKER": "allowed"},
                argv_tail=(
                    "--record",
                    str(record),
                    "--literal",
                    "$(touch should-not-exist)",
                ),
                arguments={"literal": "$(touch should-not-exist)"},
                stored_fields={
                    "text": "/result/content/0/text",
                    "public": "/result/data/public",
                    "argument": "/result/data/arguments/literal",
                },
                expected=ExpectedResult(
                    fields={"/result/content/0/text": "fixture-success"}
                ),
            )
            result = run_single_call(call)
            self.assertTrue(result.ok)
            self.assertEqual(
                dict(result.stored),
                {
                    "text": "fixture-success",
                    "public": "visible-value",
                    "argument": "$(touch should-not-exist)",
                },
            )
            self.assertNotIn("synthetic-secret", repr(result))
            observed = json.loads(record.read_text(encoding="utf-8"))
            self.assertEqual(observed["cwd"], str(root.resolve()))
            self.assertEqual(observed["marker"], "allowed")
            self.assertIsNone(observed["secret"])
            self.assertFalse((root / "should-not-exist").exists())

    def test_server_responses_have_distinct_result_kinds(self) -> None:
        cases = {
            "json-rpc-error": ResultKind.JSON_RPC_ERROR,
            "json-rpc-string-code": ResultKind.MALFORMED_RESULT,
            "tool-error": ResultKind.TOOL_ERROR,
            "malformed": ResultKind.MALFORMED_RESULT,
        }
        with tempfile.TemporaryDirectory() as directory:
            for mode, kind in cases.items():
                with self.subTest(mode=mode):
                    result = run_single_call(
                        self._call(Path(directory), mode, stored_fields={})
                    )
                    self.assertEqual(result.kind, kind)
                    self.assertNotIn("synthetic-secret", repr(result))

    def test_allowlist_cannot_be_expanded_by_server_tool_listing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = run_single_call(self._call(Path(directory), "extra-tool"))
            with self.assertRaises(ScenarioValidationError):
                SingleCall(
                    argv=(sys.executable, str(FIXTURE), "--mode", "success"),
                    cwd=Path(directory),
                    environment={},
                    tool="forbidden_tool",
                    allowed_tools=frozenset({"read_value"}),
                )

    def test_output_limit_stops_process_without_persisting_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = run_single_call(
                self._call(Path(directory), "output-limit", output_limit_bytes=1024)
            )
            self.assertEqual(result.kind, ResultKind.OUTPUT_LIMIT)

    @unittest.skipUnless(os.name != "nt", "POSIX only")
    def test_completion_and_timeout_clean_fixture_process_tree(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for mode, expected_kind, timeout in (
                ("success-tree", ResultKind.SUCCESS, 2.0),
                ("timeout", ResultKind.TIMEOUT, 1.0),
                ("parent-exits", ResultKind.MALFORMED_RESULT, 2.0),
            ):
                with self.subTest(mode=mode):
                    pid_file = root / f"{mode}.pid"
                    result = run_single_call(
                        self._call(
                            root,
                            mode,
                            argv_tail=("--pid-file", str(pid_file)),
                            timeout_seconds=timeout,
                        )
                    )
                    self.assertEqual(result.kind, expected_kind)
                    self.assertLess(result.elapsed_seconds, 4.0)
                    self._wait_for_file(pid_file)
                    self._wait_until_dead(int(pid_file.read_text(encoding="utf-8")))

    def test_cancellation_cleans_fixture_process_tree(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pid_file = root / "child.pid"
            token = CancellationToken()
            result_holder = []
            call = self._call(
                root,
                "cancel",
                argv_tail=("--pid-file", str(pid_file)),
                timeout_seconds=5,
            )
            thread = threading.Thread(
                target=lambda: result_holder.append(run_single_call(call, token))
            )
            thread.start()
            self._wait_for_file(pid_file)
            token.cancel()
            thread.join(3)
            self.assertFalse(thread.is_alive())
            self.assertEqual(result_holder[0].kind, ResultKind.CANCELLED)
            self._wait_until_dead(int(pid_file.read_text(encoding="utf-8")))

    def _wait_for_file(self, path: Path) -> None:
        deadline = time.monotonic() + 2
        while not path.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        self.assertTrue(path.exists())

    def _wait_until_dead(self, pid: int) -> None:
        deadline = time.monotonic() + 2
        while _pid_active(pid) and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertFalse(_pid_active(pid))


if __name__ == "__main__":
    unittest.main()
