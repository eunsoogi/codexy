from __future__ import annotations

import json
import subprocess
import unittest

from codexy_runtime_tools.component_inspection import doctor, status

from component_lifecycle_support import fixture


def materialize(state: fixture, *components: str, version: str = "1.3.0") -> None:
    paths = {
        "core": (".codex-plugin/plugin.json", "assets/codexy-icon.png", "agents/catalog.toml", "hooks/hooks.json"),
        "github": (".codex-plugin/plugin.json", "skills/git-workflow/SKILL.md", "agents/catalog.toml", "hooks/hooks.json"),
        "devtools": (".codex-plugin/plugin.json", ".mcp.json", "mcp/codexy-mcp-devtools"),
    }
    plugins = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}
    for component in components:
        for relative in paths[component]:
            path = state.marketplace / "plugins" / plugins[component] / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            if relative.endswith("plugin.json"):
                contents = json.dumps({"name": plugins[component], "repository": "https://github.com/eunsoogi/codexy", "version": version})
            elif relative == ".mcp.json":
                contents = json.dumps({"lsp": {"command": "./mcp/codexy-mcp-devtools"}, "codegraph": {"command": "./mcp/codexy-mcp-devtools"}})
            elif relative.endswith("hooks.json"):
                contents = json.dumps({"hooks": {"PreToolUse": []}})
            elif relative.endswith("catalog.toml"):
                contents = 'catalog_kind = "plugin-packaged-specialist-agent-files"\n'
            else:
                contents = "#!/bin/sh\n"
            path.write_text(contents, encoding="utf-8")
            if relative == "mcp/codexy-mcp-devtools":
                path.chmod(0o700)


class ComponentInspectionTests(unittest.TestCase):
    def test_status_reports_each_actual_compatible_selection_in_canonical_order(self) -> None:
        selections = (set(), {"core"}, {"core", "github"}, {"core", "devtools"}, {"core", "github", "devtools"})
        for selection in selections:
            with self.subTest(selection=selection), fixture(selection) as state:
                materialize(state, *selection)
                result = status(state.home, codex=state.codex, runner=state.run)
                self.assertEqual(result["installed_components"], [item for item in ("core", "github", "devtools") if item in selection])
                self.assertEqual(result["errors"], [])

    def test_status_uses_actual_plugins_not_recorded_selection(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.home / "getcodexy").mkdir(parents=True)
            (state.home / "getcodexy" / "installed-components.json").write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core", "github"]}), encoding="utf-8")
            result = status(state.home, codex=state.codex, runner=state.run)
        self.assertEqual(result["selected_components"], ["core", "github"])
        self.assertEqual(result["installed_components"], ["core"])
        self.assertEqual(result["inventory_consistency"], "inconsistent")

    def test_doctor_reports_healthy_missing_stale_and_incompatible_states(self) -> None:
        with self.subTest("healthy"), fixture({"core"}) as state:
            materialize(state, "core")
            self.assertEqual(doctor(state.home, codex=state.codex, runner=state.run)["component_health"], [{"component": "core", "state": "healthy"}])
        with self.subTest("stale"), fixture({"core"}, versions={"core": "1.2.0"}) as state:
            materialize(state, "core", version="1.2.0")
            self.assertEqual(doctor(state.home, codex=state.codex, runner=state.run)["component_health"][0]["state"], "stale")
        with self.subTest("missing"), fixture({"core"}) as state:
            (state.home / "getcodexy").mkdir(parents=True)
            (state.home / "getcodexy" / "installed-components.json").write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core", "github"]}), encoding="utf-8")
            materialize(state, "core")
            health = {entry["component"]: entry for entry in doctor(state.home, codex=state.codex, runner=state.run)["component_health"]}
            self.assertEqual(health["github"]["state"], "missing")
            self.assertEqual(health["github"]["repair"], "getcodexy bootstrap")
        with self.subTest("incompatible"), fixture({"core"}, versions={"core": "9.0.0"}) as state:
            materialize(state, "core", version="9.0.0")
            self.assertEqual(doctor(state.home, codex=state.codex, runner=state.run)["component_health"][0]["state"], "incompatible")

    def test_doctor_flags_ordinary_corrupt_registration_and_is_read_only(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/hooks/hooks.json").write_text("not json", encoding="utf-8")
            before = tuple(state.mutations)
            result = doctor(state.home, codex=state.codex, runner=state.run)
            self.assertEqual(tuple(state.mutations), before)
        self.assertEqual(result["component_health"][0]["state"], "incompatible")
        self.assertEqual(result["component_health"][0]["repair"], "repair the Codexy registration, then rerun getcodexy doctor")

    def test_doctor_reports_host_requirement(self) -> None:
        with fixture() as state:
            def unavailable(command: list[str]) -> subprocess.CompletedProcess[str]:
                return subprocess.CompletedProcess(command, 1, "", "unavailable")
            result = doctor(state.home, codex=state.codex, runner=unavailable)
        self.assertEqual(result["host_readiness"], {"state": "error", "missing_requirements": ["codex-plugin-list"]})
        self.assertEqual(result["errors"], [{"code": "codex-plugin-list"}])


if __name__ == "__main__":
    unittest.main()
