from __future__ import annotations

import os
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_source_admission import DiagnosticFailure, DiagnosticTree

from component_lifecycle_support import fixture
from test_component_inspection import materialize


class IntermediatePluginsTests(unittest.TestCase):
    def test_plugins_symlink_after_root_admission_fails_before_diagnostic_read(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugins, external = state.marketplace / "plugins", state.root / "external-plugins"
            original = DiagnosticTree.admits

            def replace(tree: DiagnosticTree, relatives: tuple[str, ...]) -> bool:
                plugins.rename(external)
                plugins.symlink_to(external, target_is_directory=True)
                return original(tree, relatives)

            with patch.object(DiagnosticTree, "admits", new=replace):
                result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible())

    def test_plugins_replacement_during_production_read_discards_snapshot(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugins, external = state.marketplace / "plugins", state.root / "external-plugins"
            tree = DiagnosticTree(plugins / "codexy", state.marketplace)
            self.assertTrue(tree.admits((".codex-plugin/plugin.json",)))
            original, original_lstat = os.read, os.lstat
            stable = {
                path: original_lstat(path)
                for path in (state.marketplace, tree.root, tree.root / ".codex-plugin", tree.root / ".codex-plugin/plugin.json")
            }
            replaced = False

            def replace(descriptor: int, size: int) -> bytes:
                nonlocal replaced
                if not replaced:
                    plugins.rename(external)
                    plugins.symlink_to(external, target_is_directory=True)
                    replaced = True
                return original(descriptor, size)

            def stable_anchor(path: str | Path):
                candidate = Path(path)
                return stable[candidate] if candidate in stable else original_lstat(path)

            with patch("codexy_runtime_tools.component_source_admission.os.read", side_effect=replace), patch("codexy_runtime_tools.component_source_admission.os.lstat", side_effect=stable_anchor):
                read = tree.read(".codex-plugin/plugin.json")

        self.assertTrue(replaced)
        self.assertEqual(read.failure, DiagnosticFailure.UNSAFE)

    def test_windows_reparse_plugins_after_root_admission_fails_closed(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugins, original_admits, original_lstat = state.marketplace / "plugins", DiagnosticTree.admits, os.lstat

            def reparse(path: str | Path):
                metadata = original_lstat(path)
                return SimpleNamespace(
                    st_mode=metadata.st_mode,
                    st_dev=metadata.st_dev,
                    st_ino=metadata.st_ino,
                    st_size=metadata.st_size,
                    st_mtime_ns=metadata.st_mtime_ns,
                    st_ctime_ns=metadata.st_ctime_ns,
                    st_file_attributes=0x0400 if Path(path) == plugins else 0,
                )

            def transition(tree: DiagnosticTree, relatives: tuple[str, ...]) -> bool:
                with patch("codexy_runtime_tools.component_source_admission.os.lstat", side_effect=reparse):
                    return original_admits(tree, relatives)

            with patch.object(DiagnosticTree, "admits", new=transition):
                result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible())


def _health(result: dict[str, object]) -> dict[str, object]:
    return result["component_health"][0]  # type: ignore[index]


def _incompatible() -> dict[str, str]:
    return {
        "component": "core",
        "state": "incompatible",
        "repair": "repair the Codexy registration, then rerun getcodexy doctor",
    }


if __name__ == "__main__":
    unittest.main()
