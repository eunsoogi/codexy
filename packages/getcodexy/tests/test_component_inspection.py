from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path

from codexy_runtime_tools.component_inspection import doctor, status
from codexy_runtime_tools.component_diagnostic_surfaces import CATALOGS, HOOKS, valid_surface
from codexy_runtime_tools.component_source_admission import DiagnosticTree

from component_lifecycle_support import fixture


def materialize(state: fixture, *components: str) -> None:
    paths = {
        "core": (".codex-plugin/plugin.json", "assets/codexy-icon.png"),
        "github": (".codex-plugin/plugin.json", "skills/git-workflow/SKILL.md"),
        "devtools": (".codex-plugin/plugin.json", ".mcp.json"),
    }
    plugins = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}
    for component in components:
        for relative in paths[component]:
            path = state.marketplace / "plugins" / plugins[component] / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            contents = json.dumps({"name": plugins[component], "repository": "https://github.com/eunsoogi/codexy", "version": "1.3.0"}) if relative.endswith("plugin.json") else json.dumps({"lsp": {"command": "./mcp/codexy-mcp-devtools", "args": ["lsp", "--stdio"], "cwd": "."}, "codegraph": {"command": "./mcp/codexy-mcp-devtools", "args": ["codegraph", "--stdio"], "cwd": "."}}) if relative == ".mcp.json" else "{}"
            path.write_text(contents, encoding="utf-8")
        for relative in {"core": ("agents/catalog.toml", "hooks/hooks.json"), "github": ("agents/catalog.toml", "hooks/hooks.json"), "devtools": ("mcp/codexy-mcp-devtools",)}[component]:
            path = state.marketplace / "plugins" / plugins[component] / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            contents = _catalog(component) if relative.endswith("catalog.toml") else _hooks(component) if relative.endswith("hooks.json") else "#!/bin/sh\n"
            path.write_text(contents, encoding="utf-8")
            if relative == "mcp/codexy-mcp-devtools":
                path.chmod(0o700)
        if component in CATALOGS:
            agent_root = state.marketplace / "plugins" / plugins[component] / "agents"
            for name in CATALOGS[component]["agent_files"]:
                (agent_root / name).write_text("model = \"gpt-5.6-terra\"\n", encoding="utf-8")
        if component == "core":
            for name in ("codexy-thread-delivery.sh", "codexy-thread-delivery.cmd"):
                path = state.marketplace / "plugins/codexy/hooks" / name
                path.write_text("exit 0\n", encoding="utf-8")
        if component == "github":
            for name in ("codexy-github-workflow-context.sh", "codexy-github-workflow-context.cmd", "codexy-github-admission.sh", "codexy-github-admission-issue.cmd", "codexy-github-admission-pr.cmd"):
                path = state.marketplace / "plugins/codexy-github/hooks" / name
                path.write_text("exit 0\n", encoding="utf-8")


class ComponentInspectionTests(unittest.TestCase):
    def test_packaged_registration_contract_matches_the_checked_in_plugins(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        plugins = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}
        for component, plugin in plugins.items():
            with self.subTest(component=component):
                self.assertTrue(valid_surface(DiagnosticTree(repository / "plugins" / plugin), component))

    def test_every_compatible_live_selection_is_reported_in_canonical_order(self) -> None:
        selections = (set(), {"core"}, {"core", "github"}, {"core", "devtools"}, {"core", "github", "devtools"})
        for selection in selections:
            with self.subTest(selection=selection), fixture(selection) as state:
                materialize(state, *selection)
                result = status(state.home, codex=state.codex, runner=state.run)

                expected = [component for component in ("core", "github", "devtools") if component in selection]
                self.assertEqual(result["installed_components"], expected)
                self.assertEqual(result["errors"], [])

    def test_status_uses_actual_plugins_not_recorded_selection(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.home / "getcodexy").mkdir(parents=True)
            (state.home / "getcodexy" / "installed-components.json").write_text(
                json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core", "github"]}),
                encoding="utf-8",
            )

            result = status(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(result["selected_components"], ["core", "github"])
        self.assertEqual(result["installed_components"], ["core"])
        self.assertEqual(result["inventory_consistency"], "inconsistent")
        self.assertEqual(result["errors"], [{"code": "inconsistent-installed-state"}])
        self.assertIn(("plugin", "list", "--json"), state.calls)

    def test_status_absent_inventory_is_read_only(self) -> None:
        with fixture() as state:
            self.assertFalse(state.home.exists())

            result = status(state.home, codex=state.codex, runner=state.run)

            self.assertFalse(state.home.exists())
        self.assertEqual(result["schema"], "getcodexy.status.v1")
        self.assertEqual(result["inventory"], {"state": "absent"})
        self.assertEqual(result["inventory_consistency"], "not-recorded")
        self.assertEqual(result["errors"], [])

    def test_fresh_unregistered_host_is_not_corrupt(self) -> None:
        with fixture(marketplace_present=False) as state:
            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(result["inventory_consistency"], "not-recorded")
        self.assertEqual(result["errors"], [])
        self.assertEqual(result["component_health"], [])

    def test_doctor_reports_missing_stale_and_incompatible_with_repairs(self) -> None:
        with fixture({"core", "github"}, versions={"core": "1.2.2", "github": "1.3.0"}) as state:
            materialize(state, "core", "github")
            (state.home / "getcodexy").mkdir(parents=True)
            (state.home / "getcodexy" / "installed-components.json").write_text(
                json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core", "github", "devtools"]}),
                encoding="utf-8",
            )

            result = doctor(state.home, codex=state.codex, runner=state.run)

        health = {entry["component"]: entry for entry in result["component_health"]}
        self.assertEqual(health["core"]["state"], "incompatible")
        self.assertEqual(health["github"]["state"], "incompatible")
        self.assertEqual(health["devtools"]["state"], "incompatible")
        self.assertTrue(all(entry["repair"] == "repair the Codexy registration, then rerun getcodexy doctor" for entry in health.values()))
        self.assertEqual(result["outcome"], "completed")

    def test_doctor_accepts_the_supported_devtools_dispatcher_without_executing_it(self) -> None:
        with fixture({"core", "devtools"}) as state:
            materialize(state, "core", "devtools")
            wrapper = state.marketplace / "plugins/codexy-devtools/mcp/codexy-mcp-devtools"
            wrapper.write_text("exec uvx --from getcodexy==1.2.2 codexy-mcp-runtime\n", encoding="utf-8")

            result = doctor(state.home, codex=state.codex, runner=state.run)

        health = {entry["component"]: entry["state"] for entry in result["component_health"]}
        self.assertEqual(health, {"core": "healthy", "devtools": "healthy"})

    def test_doctor_flags_malformed_hook_configuration_as_incompatible(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/hooks/hooks.json").write_text("not json", encoding="utf-8")

            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(result["component_health"][0]["state"], "incompatible")
        self.assertEqual(result["component_health"][0]["repair"], "repair the Codexy registration, then rerun getcodexy doctor")

    def test_doctor_flags_malformed_manifest_and_legacy_core_monolith(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/.codex-plugin/plugin.json").write_text("not json", encoding="utf-8")
            (state.marketplace / "plugins/codexy/mcp").mkdir()

            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(result["component_health"][0]["state"], "incompatible")

    def test_malformed_unregistered_inventory_is_not_silently_accepted(self) -> None:
        with fixture(marketplace_present=False, inventory_override={"installed": [{"name": 123}]}) as state:
            result = status(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(result["inventory_consistency"], "inconsistent")
        self.assertEqual(result["errors"], [{"code": "invalid-installed-inventory"}])

    def test_doctor_does_not_execute_mcp_or_change_home(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            before = tuple(state.mutations)
            result = doctor(state.home, codex=state.codex, runner=state.run)

            self.assertEqual(tuple(state.mutations), before)
            self.assertFalse(any("mcp" in part for call in state.calls for part in call))
        self.assertEqual(result["component_health"], [{"component": "core", "state": "healthy"}])

    def test_host_failure_is_an_actionable_doctor_result(self) -> None:
        with fixture() as state:
            def unavailable(command: list[str]) -> subprocess.CompletedProcess[str]:
                return subprocess.CompletedProcess(command, 1, "", "unavailable")

            result = doctor(state.home, codex=state.codex, runner=unavailable)

        self.assertEqual(result["host_readiness"], {"state": "error", "missing_requirements": ["codex-plugin-list"]})
        self.assertEqual(result["errors"], [{"code": "codex-plugin-list"}])

    def test_doctor_distinguishes_plugin_list_and_marketplace_probe_failures(self) -> None:
        for tail, expected in (("plugin", "list", "--json"), "codex-plugin-list"), (("plugin", "marketplace", "list", "--json"), "codex-marketplace-list"):
            with self.subTest(tail=tail), fixture() as state:
                def failing(command: list[str]) -> subprocess.CompletedProcess[str]:
                    if tuple(command[1:]) == tail:
                        return subprocess.CompletedProcess(command, 1, "", "unavailable")
                    return state.run(command)

                result = doctor(state.home, codex=state.codex, runner=failing)

            self.assertEqual(result["host_readiness"]["missing_requirements"], [expected])
            self.assertEqual(result["errors"], [{"code": expected}])

    def test_marketplace_probe_failure_preserves_populated_plugin_observation(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            def failing(command: list[str]) -> subprocess.CompletedProcess[str]:
                if tuple(command[1:]) == ("plugin", "marketplace", "list", "--json"):
                    return subprocess.CompletedProcess(command, 1, "", "unavailable")
                return state.run(command)

            observed = status(state.home, codex=state.codex, runner=failing)
            result = doctor(state.home, codex=state.codex, runner=failing)

        self.assertEqual(observed["installed_components"], ["core"])
        self.assertEqual(observed["errors"], [{"code": "codex-marketplace-list"}])
        self.assertEqual(result["component_health"], [{"component": "core", "state": "incompatible", "repair": "repair the Codexy registration, then rerun getcodexy doctor"}])
        self.assertEqual(result["host_readiness"], {"state": "error", "missing_requirements": ["codex-marketplace-list"]})

    def test_doctor_requires_canonical_catalog_hooks_and_mcp_bindings(self) -> None:
        cases = (({"core"}, "core", "agents/catalog.toml", "# comments only\n"), ({"core"}, "core", "hooks/hooks.json", '{"hooks":{"Other":[]}}'), ({"core", "devtools"}, "devtools", ".mcp.json", '{"lsp":{"command":"./mcp/codexy-mcp-devtools","args":["lsp","--stdio"],"cwd":"."}}'))
        for selection, component, relative, contents in cases:
            with self.subTest(component=component, relative=relative), fixture(selection) as state:
                materialize(state, *selection)
                (state.marketplace / "plugins" / {"core": "codexy", "devtools": "codexy-devtools"}[component] / relative).write_text(contents, encoding="utf-8")

                result = doctor(state.home, codex=state.codex, runner=state.run)

            health = {entry["component"]: entry for entry in result["component_health"]}
            self.assertEqual(health[component], {"component": component, "state": "incompatible", "repair": "repair the Codexy registration, then rerun getcodexy doctor"})

    def test_doctor_compares_versions_in_both_directions(self) -> None:
        expected = {"1.2.9": "stale", "1.3.0": "healthy", "1.3.1": "incompatible", "1.3.0-alpha.1": "incompatible"}
        for version, health in expected.items():
            with self.subTest(version=version), fixture({"core"}, versions={"core": version}) as state:
                materialize(state, "core")
                result = doctor(state.home, codex=state.codex, runner=state.run)

            self.assertEqual(result["component_health"][0]["state"], health)

    def test_doctor_requires_registered_hook_targets(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/hooks/codexy-thread-delivery.sh").unlink()

            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(result["component_health"], [{"component": "core", "state": "incompatible", "repair": "repair the Codexy registration, then rerun getcodexy doctor"}])

def _catalog(component: str) -> str:
    values = CATALOGS[component]
    return "\n".join(f'{key} = {json.dumps(value)}' for key, value in values.items()) + "\n"


def _hooks(component: str) -> str:
    return json.dumps(HOOKS[component])


if __name__ == "__main__":
    unittest.main()
