from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path

from codexy_runtime_tools.component_inspection import doctor, status

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
            contents = json.dumps({"name": plugins[component], "repository": "https://github.com/eunsoogi/codexy", "version": "1.3.0"}) if relative.endswith("plugin.json") else json.dumps({"lsp": {"command": "./mcp/codexy-mcp-devtools"}}) if relative == ".mcp.json" else "{}"
            path.write_text(contents, encoding="utf-8")
        for relative in {"core": ("agents/catalog.toml", "hooks/hooks.json"), "github": ("agents/catalog.toml", "hooks/hooks.json"), "devtools": ("mcp/codexy-mcp-devtools",)}[component]:
            path = state.marketplace / "plugins" / plugins[component] / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            contents = json.dumps({"hooks": {"PreToolUse": [{"command": "hooks/entry.sh"}]}}) if relative.endswith("hooks.json") else "#!/bin/sh\n"
            path.write_text(contents, encoding="utf-8")
            if relative == "mcp/codexy-mcp-devtools":
                path.chmod(0o700)


class ComponentInspectionTests(unittest.TestCase):
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
        self.assertEqual(health["core"]["state"], "stale")
        self.assertEqual(health["github"]["state"], "incompatible")
        self.assertEqual(health["devtools"]["state"], "missing")
        self.assertEqual(health["devtools"]["repair"], "getcodexy bootstrap")
        self.assertEqual(result["outcome"], "completed")

    def test_doctor_accepts_the_supported_devtools_dispatcher_without_executing_it(self) -> None:
        with fixture({"core", "devtools"}) as state:
            materialize(state, "core", "devtools")
            wrapper = state.marketplace / "plugins/codexy-devtools/mcp/codexy-mcp-devtools"
            wrapper.write_text("exec uvx --from getcodexy==1.2.2 codexy-mcp-runtime\n", encoding="utf-8")

            result = doctor(state.home, codex=state.codex, runner=state.run)

        health = {entry["component"]: entry["state"] for entry in result["component_health"]}
        self.assertEqual(health, {"core": "healthy", "devtools": "healthy"})

    def test_doctor_flags_malformed_hook_configuration_as_stale(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/hooks/hooks.json").write_text("not json", encoding="utf-8")

            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(result["component_health"][0]["state"], "stale")
        self.assertEqual(result["component_health"][0]["repair"], "getcodexy bootstrap")

    def test_doctor_flags_malformed_manifest_and_legacy_core_monolith(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/.codex-plugin/plugin.json").write_text("not json", encoding="utf-8")
            (state.marketplace / "plugins/codexy/mcp").mkdir()

            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(result["component_health"][0]["state"], "stale")

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

        self.assertEqual(result["host_readiness"], {"state": "missing", "missing_requirements": ["codex-marketplace-list"]})
        self.assertEqual(result["errors"], [{"code": "invalid-installed-inventory"}])


if __name__ == "__main__":
    unittest.main()
