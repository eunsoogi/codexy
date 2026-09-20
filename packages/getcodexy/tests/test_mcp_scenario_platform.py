"""Platform-boundary coverage for the bounded MCP scenario core."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


REPOSITORY = Path(__file__).resolve().parents[3]
SCENARIO_SCRIPTS = REPOSITORY / ".agents/skills/mcp-test/scripts"
FIXTURE = Path(__file__).with_name("mcp_scenario_fixtures") / "fixture_server.py"
if str(SCENARIO_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCENARIO_SCRIPTS))

from scenario_core import (  # noqa: E402
    SingleCall,
    UnsupportedPlatformError,
    run_single_call,
    supported_versions,
)


class PlatformBoundaryTests(unittest.TestCase):
    def test_unsupported_platform_is_rejected_before_process_launch(self) -> None:
        support = supported_versions()
        self.assertEqual(support["platforms"], ["posix"])
        self.assertIn("#1148", support["platform_limitations"][0])
        with tempfile.TemporaryDirectory() as directory:
            call = SingleCall(
                argv=(sys.executable, "-u", str(FIXTURE), "--mode", "success"),
                cwd=Path(directory),
                environment={},
                tool="read_value",
                allowed_tools=frozenset({"read_value"}),
                stored_fields={},
            )
            with (
                patch("scenario_core.support.os.name", "nt"),
                patch(
                    "scenario_core.process.subprocess.Popen",
                    side_effect=AssertionError("unsupported platform was launched"),
                ),
            ):
                with self.assertRaises(UnsupportedPlatformError):
                    run_single_call(call)


if __name__ == "__main__":
    unittest.main()
