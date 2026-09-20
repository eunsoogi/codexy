"""Host-probe and interpreter-compatibility inspection scenarios."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_manifest import load_component_manifest

from component_lifecycle_support import fixture


class ComponentInspectionHostCases:
    def test_doctor_reports_standalone_registration_states_read_only(self) -> None:
        mutations = {
            "exact": None,
            "stale": "stale",
            "missing": "missing",
            "unmanaged-conflict": "conflict",
        }
        for expected, mutation in mutations.items():
            with self.subTest(state=expected), fixture({"core"}) as state:
                materialize = state.marketplace / "plugins/codexy"
                _register_core(state, materialize)
                role = state.home / "agents/codexy/codexy-sentinel.toml"
                if mutation == "stale":
                    role.write_text(
                        role.read_text(encoding="utf-8") + "# stale\n", encoding="utf-8"
                    )
                elif mutation == "missing":
                    role.unlink()
                elif mutation == "conflict":
                    role.write_text('name = "codexy-sentinel"\n', encoding="utf-8")
                before = _home_snapshot(state.home)
                result = doctor(state.home, codex=state.codex, runner=state.run)
                self.assertEqual(_home_snapshot(state.home), before)
            entry = result["component_health"][0]
            registration = entry["observed"]["registration"]
            self.assertTrue(registration["observed"])
            self.assertEqual(registration["expected_count"], 8)
            self.assertEqual(registration["state"], expected)
            self.assertEqual(
                entry["state"],
                {
                    "exact": "healthy",
                    "stale": "stale",
                    "missing": "missing",
                    "unmanaged-conflict": "incompatible",
                }[expected],
            )
            self.assertEqual(
                next(
                    item for item in registration["roles"] if item["file"] == role.name
                )["state"],
                "exact" if expected == "exact" else expected,
            )

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


def _register_core(state, plugin: Path) -> None:
    script = plugin / "skills/orchestration/scripts/register_codexy_agents.py"
    result = subprocess.run(
        [
            sys.executable,
            str(script),
            "--plugin-root",
            str(plugin),
            "--codex-home",
            str(state.home),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        raise AssertionError(result.stderr)


def _home_snapshot(home: Path) -> tuple[tuple[str, bytes], ...]:
    return tuple(
        sorted(
            (path.relative_to(home).as_posix(), path.read_bytes())
            for path in home.rglob("*")
            if path.is_file()
        )
    )
