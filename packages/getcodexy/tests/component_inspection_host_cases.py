"""Host-probe and interpreter-compatibility inspection scenarios."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_manifest import load_component_manifest

from component_lifecycle_support import fixture


class ComponentInspectionHostCases:
    def test_doctor_reports_host_requirement(self) -> None:
        with fixture() as state:

            def unavailable(command: list[str]) -> subprocess.CompletedProcess[str]:
                return subprocess.CompletedProcess(command, 1, "", "unavailable")

            result = doctor(state.home, codex=state.codex, runner=unavailable)
        self.assertEqual(
            result["host_readiness"],
            {"state": "error", "missing_requirements": ["codex-plugin-list"]},
        )
        self.assertEqual(result["errors"], [{"code": "invalid-installed-inventory"}])

    def test_reports_keep_host_probe_detail_outside_the_closed_domain_error_set(
        self,
    ) -> None:
        with fixture() as state:

            def unavailable(command: list[str]) -> subprocess.CompletedProcess[str]:
                return subprocess.CompletedProcess(command, 1, "", "unavailable")

            result = doctor(state.home, codex=state.codex, runner=unavailable)
        self.assertEqual(
            result["host_readiness"]["missing_requirements"], ["codex-plugin-list"]
        )
        self.assertEqual(
            {error["code"] for error in result["errors"]},
            {"invalid-installed-inventory"},
        )
        self.assertTrue(
            {error["code"] for error in result["errors"]}.issubset(
                load_component_manifest().domain_errors
            )
        )

    def test_registration_checks_import_without_the_python_311_tomllib_module(
        self,
    ) -> None:
        source = Path(__file__).resolve().parents[1] / "src"
        result = subprocess.run(
            [
                sys.executable,
                "-c",
                "import sys; sys.modules['tomllib'] = None; import codexy_runtime_tools.component_registration_health",
            ],
            env={"PYTHONPATH": str(source)},
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
