from __future__ import annotations

import json
import unittest

from codexy_runtime_tools.component_diagnostic_surfaces import valid_surface
from codexy_runtime_tools.component_inspection import doctor

from component_lifecycle_support import fixture
from test_component_inspection import materialize


class DiagnosticSurfaceTests(unittest.TestCase):
    def test_canonical_managed_registrations_are_healthy(self) -> None:
        selections = ({"core"}, {"core", "github"}, {"core", "devtools"}, {"core", "github", "devtools"})
        for selection in selections:
            with self.subTest(selection=selection), fixture(selection) as state:
                materialize(state, *selection)
                result = doctor(state.home, codex=state.codex, runner=state.run)

            health = {entry["component"]: entry["state"] for entry in result["component_health"]}
            self.assertEqual(health, {component: "healthy" for component in selection})

    def test_managed_registration_files_fail_closed_for_symlink_and_special_paths(self) -> None:
        cases = (
            ({"core"}, "core", "agents/catalog.toml", "symlink"),
            ({"core"}, "core", "hooks/hooks.json", "symlink"),
            ({"core", "github"}, "github", "agents/catalog.toml", "symlink"),
            ({"core", "github"}, "github", "hooks/hooks.json", "symlink"),
            ({"core", "devtools"}, "devtools", "mcp/codexy-mcp-devtools", "symlink"),
            ({"core", "devtools"}, "devtools", ".mcp.json", "symlink"),
            ({"core"}, "core", "agents/catalog.toml", "directory"),
            ({"core"}, "core", "hooks/hooks.json", "directory"),
            ({"core", "github"}, "github", "agents/catalog.toml", "directory"),
            ({"core", "github"}, "github", "hooks/hooks.json", "directory"),
            ({"core", "devtools"}, "devtools", "mcp/codexy-mcp-devtools", "directory"),
            ({"core", "devtools"}, "devtools", ".mcp.json", "directory"),
        )
        plugins = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}
        for selection, component, relative, kind in cases:
            with self.subTest(component=component, path=relative, kind=kind), fixture(selection) as state:
                materialize(state, *selection)
                path = state.marketplace / "plugins" / plugins[component] / relative
                path.unlink()
                if kind == "symlink":
                    path.symlink_to(state.marketplace / "plugins" / plugins[component] / ".codex-plugin/plugin.json")
                else:
                    path.mkdir()

                result = doctor(state.home, codex=state.codex, runner=state.run)
                self.assertFalse(valid_surface(state.marketplace / "plugins" / plugins[component], component))

            health = {entry["component"]: entry for entry in result["component_health"]}
            self.assertEqual(health[component], {"component": component, "state": "stale", "repair": "getcodexy bootstrap"})

    def test_devtools_mcp_requires_exact_lsp_and_codegraph_bindings(self) -> None:
        bindings = (
            {"lsp": {"command": "./mcp/codexy-mcp-devtools", "args": ["lsp"], "cwd": "."}, "codegraph": {"command": "./mcp/codexy-mcp-devtools", "args": ["codegraph", "--stdio"], "cwd": "."}},
            {"lsp": {"command": "./mcp/codexy-mcp-devtools", "args": ["lsp", "--stdio"], "cwd": "."}, "codegraph": {"command": "./mcp/not-codexy", "args": ["codegraph", "--stdio"], "cwd": "."}},
        )
        for binding in bindings:
            with self.subTest(binding=binding), fixture({"core", "devtools"}) as state:
                materialize(state, "core", "devtools")
                plugin = state.marketplace / "plugins/codexy-devtools"
                (plugin / ".mcp.json").write_text(json.dumps(binding), encoding="utf-8")
                result = doctor(state.home, codex=state.codex, runner=state.run)

                self.assertFalse(valid_surface(plugin, "devtools"))
            health = {entry["component"]: entry for entry in result["component_health"]}
            self.assertEqual(health["devtools"], {"component": "devtools", "state": "stale", "repair": "getcodexy bootstrap"})


if __name__ == "__main__":
    unittest.main()
