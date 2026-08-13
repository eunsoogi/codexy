from __future__ import annotations

import os
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from codexy_runtime_tools.component_inspection import doctor
import codexy_runtime_tools.component_source_admission as source
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

    def test_plugins_swap_and_restore_between_snapshots_discards_substituted_bytes(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugins = state.marketplace / "plugins"
            saved, malicious = state.root / "saved-plugins", state.root / "malicious-plugins"
            tree = DiagnosticTree(plugins / "codexy", state.marketplace)
            original_metadata, original_open = source._tree_metadata, source._open_regular
            original_lstat = os.lstat
            stable = {
                path: original_lstat(path)
                for path in (
                    state.marketplace,
                    plugins,
                    tree.root,
                    tree.root / ".codex-plugin",
                    tree.root / ".codex-plugin/plugin.json",
                )
            }
            expected_identity = source._tree_identity(tree.root, Path(".codex-plugin/plugin.json"), tree.anchor)
            calls, swapped, restored = 0, False, False

            def swap_before_second_snapshot(*args: object):
                nonlocal calls, swapped
                calls += 1
                if calls == 2:
                    plugins.rename(saved)
                    target = plugins / "codexy/.codex-plugin/plugin.json"
                    target.parent.mkdir(parents=True)
                    target.write_text('{"name":"substituted"}', encoding="utf-8")
                    swapped = True
                return original_metadata(*args)

            def open_then_restore(*args: object) -> int:
                nonlocal restored
                descriptor = original_open(*args)
                plugins.rename(malicious)
                saved.rename(plugins)
                restored = True
                return descriptor

            def stable_after_restore(path: str | Path):
                candidate = Path(path)
                return stable[candidate] if restored and candidate in stable else original_lstat(path)

            def stable_identity(*args: object):
                source._tree_metadata(*args)
                return expected_identity

            try:
                with patch("codexy_runtime_tools.component_source_admission._tree_metadata", side_effect=swap_before_second_snapshot), patch("codexy_runtime_tools.component_source_admission._tree_identity", side_effect=stable_identity), patch("codexy_runtime_tools.component_source_admission._open_regular", side_effect=open_then_restore), patch("codexy_runtime_tools.component_source_admission.os.lstat", side_effect=stable_after_restore):
                    read = tree.read(".codex-plugin/plugin.json")
            finally:
                if saved.exists():
                    plugins.rename(malicious)
                    saved.rename(plugins)

        self.assertTrue(swapped)
        self.assertEqual(read.failure, DiagnosticFailure.CHANGED)

    @unittest.skipIf(os.name == "nt", "POSIX descriptor walk")
    def test_production_read_opens_every_component_path_segment_from_marketplace_anchor(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            tree = DiagnosticTree(state.marketplace / "plugins/codexy", state.marketplace)
            original_open, opened = os.open, []

            def observe(path: str | bytes | os.PathLike[str], *args: object, **kwargs: object) -> int:
                opened.append((Path(path), kwargs.get("dir_fd")))
                return original_open(path, *args, **kwargs)

            with patch("codexy_runtime_tools.component_source_admission.os.open", side_effect=observe):
                read = tree.read(".codex-plugin/plugin.json")

        self.assertIsNone(read.failure)
        self.assertEqual(opened[0][0], state.marketplace)
        self.assertEqual([path for path, _ in opened[1:]], [Path("plugins"), Path("codexy"), Path(".codex-plugin"), Path("plugin.json")])


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
