"""Doctor reports marker-owned role projections without installed components."""

from __future__ import annotations

import subprocess
import sys
import unittest

from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.updater import sync_agents
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import fixture


class ComponentInspectionResidualRegistrationTests(unittest.TestCase):
    def test_doctor_ignores_unmarked_roles_for_absent_components(self) -> None:
        with fixture() as state:
            core_role = state.home / "agents/codexy/personal.toml"
            github_role = state.home / "agents/codexy-github/personal.toml"
            core_role.parent.mkdir(parents=True)
            github_role.parent.mkdir(parents=True)
            core_bytes = b'name = "personal core role"\n'
            github_bytes = b'name = "personal GitHub role"\n'
            core_role.write_bytes(core_bytes)
            github_role.write_bytes(github_bytes)
            record(state.home, [])

            report = doctor(state.home, codex=state.codex, runner=state.run)
            # Ownership comes from the Codexy marker, so user roles stay out of doctor failures.
            self.assertEqual(report["component_health"], [])
            self.assertEqual(core_role.read_bytes(), core_bytes)
            self.assertEqual(github_role.read_bytes(), github_bytes)

    def test_doctor_reports_core_and_github_roles_with_empty_inventory_read_only(
        self,
    ) -> None:
        with fixture() as state:
            core = state.marketplace / "plugins/codexy"
            github_script = (
                state.marketplace
                / "plugins/codexy-github/skills/git-workflow/scripts/bootstrap_codexy_github_agent.py"
            )
            self.assertEqual(
                sync_agents(core, state.home, "install").status, "completed"
            )
            installed = subprocess.run(
                [
                    sys.executable,
                    "-B",
                    str(github_script),
                    "--codex-home",
                    str(state.home),
                ],
                capture_output=True,
                text=True,
                check=False,
                env={"PYTHONNOUSERSITE": "1"},
            )
            self.assertEqual(installed.returncode, 0, installed.stderr)
            record(state.home, [])
            roles = {
                path.relative_to(state.home): path.read_bytes()
                for root in (
                    state.home / "agents/codexy",
                    state.home / "agents/codexy-github",
                )
                for path in root.glob("*.toml")
            }

            report = doctor(state.home, codex=state.codex, runner=state.run)

            after = {
                path.relative_to(state.home): path.read_bytes()
                for root in (
                    state.home / "agents/codexy",
                    state.home / "agents/codexy-github",
                )
                for path in root.glob("*.toml")
            }

        health = {entry["component"]: entry for entry in report["component_health"]}
        self.assertEqual(set(health), {"core", "github"})
        for component, entry in health.items():
            self.assertFalse(entry["healthy"])
            self.assertEqual(entry["reason_code"], "orphaned-managed-registration")
            self.assertIn("remove", entry["repair"])
            self.assertTrue(entry["observed"]["registration"]["managed_extra"])
        self.assertEqual(report["inventory"]["components"], [])
        self.assertEqual(report["errors"], [])
        self.assertEqual(after, roles)

    def test_doctor_detects_crlf_managed_role_for_absent_component(self) -> None:
        with fixture() as state:
            github_role = state.home / "agents/codexy-github/legacy-weaver.toml"
            github_role.parent.mkdir(parents=True)
            role_bytes = b"# Managed by Codexy GitHub.\r\nname = 'legacy-weaver'\r\n"
            github_role.write_bytes(role_bytes)
            record(state.home, [])

            report = doctor(state.home, codex=state.codex, runner=state.run)
            self.assertEqual(github_role.read_bytes(), role_bytes)

        health = {entry["component"]: entry for entry in report["component_health"]}
        self.assertEqual(
            health["github"]["reason_code"], "orphaned-managed-registration"
        )
        self.assertFalse(health["github"]["healthy"])
        self.assertEqual(report["inventory"]["components"], [])
