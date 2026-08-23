from __future__ import annotations

import json
import shutil
import subprocess
import sys
import unittest
from pathlib import Path

from codexy_runtime_tools.component_inspection import doctor, status
from codexy_runtime_tools.component_manifest import load_component_manifest
from component_lifecycle_support import fixture
from component_inspection_host_cases import ComponentInspectionHostCases


def materialize(
    state: fixture, *components: str, version: str = load_component_manifest().version
) -> None:
    plugins = {
        "core": "codexy",
        "github": "codexy-github",
        "devtools": "codexy-devtools",
    }
    repository = Path(__file__).resolve().parents[3]
    for component in components:
        root = state.marketplace / "plugins" / plugins[component]
        if root.exists():
            continue
        shutil.copytree(repository / "plugins" / plugins[component], root)
        manifest = root / ".codex-plugin/plugin.json"
        contents = json.loads(manifest.read_text(encoding="utf-8"))
        contents["version"] = version
        manifest.write_text(json.dumps(contents), encoding="utf-8")


class ComponentInspectionTests(ComponentInspectionHostCases, unittest.TestCase):
    def test_status_reports_each_actual_compatible_selection_in_canonical_order(
        self,
    ) -> None:
        selections = (
            set(),
            {"core"},
            {"core", "github"},
            {"core", "devtools"},
            {"core", "github", "devtools"},
        )
        for selection in selections:
            with self.subTest(selection=selection), fixture(selection) as state:
                materialize(state, *selection)
                result = status(state.home, codex=state.codex, runner=state.run)
                self.assertEqual(
                    result["installed_components"],
                    [
                        item
                        for item in ("core", "github", "devtools")
                        if item in selection
                    ],
                )
                self.assertEqual(result["errors"], [])

    def test_status_uses_actual_plugins_not_recorded_selection(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.home / "getcodexy").mkdir(parents=True)
            (state.home / "getcodexy" / "installed-components.json").write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.installed-component-inventory.v1",
                        "components": ["core", "github"],
                    }
                ),
                encoding="utf-8",
            )
            result = status(state.home, codex=state.codex, runner=state.run)
        self.assertEqual(result["selected_components"], ["core", "github"])
        self.assertEqual(result["installed_components"], ["core"])
        self.assertEqual(result["inventory_consistency"], "inconsistent")

    def test_doctor_reports_healthy_missing_stale_and_incompatible_states(self) -> None:
        with self.subTest("healthy"), fixture({"core"}) as state:
            materialize(state, "core")
            self.assertEqual(
                doctor(state.home, codex=state.codex, runner=state.run)[
                    "component_health"
                ],
                [{"component": "core", "state": "healthy"}],
            )
        with (
            self.subTest("stale"),
            fixture({"core"}, versions={"core": "1.2.0"}) as state,
        ):
            materialize(state, "core", version="1.2.0")
            self.assertEqual(
                doctor(state.home, codex=state.codex, runner=state.run)[
                    "component_health"
                ][0]["state"],
                "stale",
            )
        with self.subTest("missing"), fixture({"core"}) as state:
            (state.home / "getcodexy").mkdir(parents=True)
            (state.home / "getcodexy" / "installed-components.json").write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.installed-component-inventory.v1",
                        "components": ["core", "github"],
                    }
                ),
                encoding="utf-8",
            )
            materialize(state, "core")
            health = {
                entry["component"]: entry
                for entry in doctor(state.home, codex=state.codex, runner=state.run)[
                    "component_health"
                ]
            }
            self.assertEqual(health["github"]["state"], "missing")
            self.assertEqual(health["github"]["repair"], "getcodexy bootstrap")
        with (
            self.subTest("incompatible"),
            fixture({"core"}, versions={"core": "9.0.0"}) as state,
        ):
            materialize(state, "core", version="9.0.0")
            self.assertEqual(
                doctor(state.home, codex=state.codex, runner=state.run)[
                    "component_health"
                ][0]["state"],
                "incompatible",
            )

    def test_doctor_flags_ordinary_corrupt_registration_and_is_read_only(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/hooks/hooks.json").write_text(
                "not json", encoding="utf-8"
            )
            before = tuple(state.mutations)
            result = doctor(state.home, codex=state.codex, runner=state.run)
            self.assertEqual(tuple(state.mutations), before)
        self.assertEqual(result["component_health"][0]["state"], "incompatible")
        self.assertEqual(
            result["component_health"][0]["repair"],
            "repair the Codexy registration, then rerun getcodexy doctor",
        )

    def test_doctor_rejects_noncanonical_catalog_hook_mcp_and_launcher_bindings(
        self,
    ) -> None:
        cases = (
            (
                "core",
                "agents/catalog.toml",
                'catalog_kind = "plugin-packaged-specialist-agent-files"\n',
            ),
            ("core", "hooks/hooks.json", json.dumps({"hooks": {"PreToolUse": []}})),
            (
                "devtools",
                ".mcp.json",
                json.dumps(
                    {
                        "lsp": {"command": "./mcp/unrelated"},
                        "codegraph": {"command": "./mcp/unrelated"},
                    }
                ),
            ),
        )
        plugins = {"core": "codexy", "devtools": "codexy-devtools"}
        for component, relative, contents in cases:
            with (
                self.subTest(component=component, relative=relative),
                fixture({"core", component}) as state,
            ):
                materialize(state, "core", component)
                (
                    state.marketplace / "plugins" / plugins[component] / relative
                ).write_text(contents, encoding="utf-8")
                result = doctor(state.home, codex=state.codex, runner=state.run)
            health = {
                entry["component"]: entry["state"]
                for entry in result["component_health"]
            }
            self.assertEqual(health[component], "incompatible")

    def test_doctor_rejects_missing_or_tampered_canonical_hook_dependencies(
        self,
    ) -> None:
        dependencies = (
            "hooks/codexy-thread-delivery.sh",
            "hooks/codexy-thread-delivery.cmd",
            "hooks/codexy-child-thread-creation.sh",
            "hooks/codexy-child-thread-creation.cmd",
            "hooks/codexy-child-thread-creation.py",
            "hooks/codexy_policy/child_thread_creation.py",
            "hooks/codexy_policy/envelope.py",
        )
        for relative in dependencies:
            for mutation in ("missing", "tampered"):
                with (
                    self.subTest(relative=relative, mutation=mutation),
                    fixture({"core"}) as state,
                ):
                    materialize(state, "core")
                    path = state.marketplace / "plugins/codexy" / relative
                    if mutation == "missing":
                        path.unlink()
                    else:
                        path.write_bytes(path.read_bytes() + b"\n# tampered\n")
                    result = doctor(state.home, codex=state.codex, runner=state.run)
                self.assertEqual(result["component_health"][0]["state"], "incompatible")


if __name__ == "__main__":
    unittest.main()
