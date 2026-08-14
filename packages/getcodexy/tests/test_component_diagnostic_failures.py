from __future__ import annotations

import json
import os
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_source_admission import (
    DiagnosticFailure,
    DiagnosticTree,
    _ChangedDiagnosticPath,
)

from component_lifecycle_support import fixture
from test_component_inspection import materialize


class DiagnosticFailureTests(unittest.TestCase):
    def test_unsafe_optional_legacy_path_is_incompatible_not_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugin = state.marketplace / "plugins/codexy"
            legacy = plugin / ".mcp.json"
            legacy.symlink_to(plugin / ".codex-plugin/plugin.json")
            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_post_admission_read_failure_is_incompatible_not_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            with patch("codexy_runtime_tools.component_source_admission._open_regular", side_effect=OSError("swapped")):
                result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_malformed_manifest_is_incompatible_not_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/.codex-plugin/plugin.json").write_text("not json", encoding="utf-8")
            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_duplicate_manifest_keys_are_incompatible_not_healthy(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/.codex-plugin/plugin.json").write_text(
                '{"name":"wrong","name":"codexy","repository":"https://github.com/eunsoogi/codexy","version":"1.3.0"}',
                encoding="utf-8",
            )
            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_malformed_hook_registration_is_incompatible_not_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/hooks/hooks.json").write_text("not json", encoding="utf-8")
            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_reparse_ancestor_above_marketplace_is_incompatible(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            original = __import__("os").lstat

            def reparse(path: str | Path):
                metadata = original(path)
                attributes = 0x0400 if Path(path) == state.marketplace.parent else 0
                return SimpleNamespace(
                    st_mode=metadata.st_mode,
                    st_dev=metadata.st_dev,
                    st_ino=metadata.st_ino,
                    st_size=metadata.st_size,
                    st_file_attributes=attributes,
                )

            with patch("codexy_runtime_tools.component_source_admission.os.lstat", side_effect=reparse):
                result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_read_taxonomy_retains_missing_unsafe_changed_and_unreadable_causes(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            tree = DiagnosticTree(state.marketplace / "plugins/codexy")
            self.assertEqual(tree.read("not-present").failure, DiagnosticFailure.MISSING)
            unsafe = state.marketplace / "plugins/codexy/.mcp.json"
            unsafe.symlink_to(state.marketplace / "plugins/codexy/.codex-plugin/plugin.json")
            self.assertEqual(tree.read(".mcp.json").failure, DiagnosticFailure.UNSAFE)
            with patch("codexy_runtime_tools.component_source_admission._read_regular", side_effect=_ChangedDiagnosticPath("swapped")):
                self.assertEqual(tree.read("hooks/hooks.json").failure, DiagnosticFailure.CHANGED)
            with patch("codexy_runtime_tools.component_source_admission._read_regular", side_effect=PermissionError("denied")):
                self.assertEqual(tree.read("hooks/hooks.json").failure, DiagnosticFailure.UNREADABLE)

    def test_duplicate_toml_catalog_is_incompatible_not_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            (state.marketplace / "plugins/codexy/agents/catalog.toml").write_text('version = "0.1.0"\nversion = "0.1.0"\n', encoding="utf-8")
            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_coherent_older_manifest_is_stale_but_unsafe_older_manifest_is_incompatible(self) -> None:
        with fixture({"core"}, versions={"core": "1.2.0"}) as state:
            materialize(state, "core")
            manifest = state.marketplace / "plugins/codexy/.codex-plugin/plugin.json"
            value = json.loads(manifest.read_text(encoding="utf-8"))
            value["version"] = "1.2.0"
            manifest.write_text(json.dumps(value), encoding="utf-8")
            coherent = doctor(state.home, codex=state.codex, runner=state.run)
            manifest.write_text("not json", encoding="utf-8")
            unsafe = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(coherent), {"component": "core", "state": "stale", "repair": "getcodexy bootstrap"})
        self.assertEqual(_health(unsafe), _incompatible("core"))

    def test_nonfinite_manifest_json_is_incompatible_not_healthy(self) -> None:
        for constant in ("NaN", "Infinity", "-Infinity", "1e999", "-1e999"):
            with self.subTest(constant=constant), fixture({"core"}) as state:
                materialize(state, "core")
                (state.marketplace / "plugins/codexy/.codex-plugin/plugin.json").write_text(
                    '{"name":"codexy","repository":"https://github.com/eunsoogi/codexy","version":"1.3.0","extra":{"nested":' + constant + '}}',
                    encoding="utf-8",
                )
                result = doctor(state.home, codex=state.codex, runner=state.run)

            self.assertEqual(_health(result), _incompatible("core"))

    def test_nonfinite_older_manifest_json_is_incompatible_not_bootstrap(self) -> None:
        with fixture({"core"}, versions={"core": "1.2.0"}) as state:
            materialize(state, "core", version="1.2.0")
            (state.marketplace / "plugins/codexy/.codex-plugin/plugin.json").write_text(
                '{"name":"codexy","repository":"https://github.com/eunsoogi/codexy","version":"1.2.0","extra":{"nested":1e999}}',
                encoding="utf-8",
            )
            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_deep_manifest_json_is_incompatible_not_process_failure(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            manifest = state.marketplace / "plugins/codexy/.codex-plugin/plugin.json"
            manifest.write_text(
                '{"name":"codexy","repository":"https://github.com/eunsoogi/codexy","version":"1.3.0","extra":'
                + "[" * 2_000
                + "0"
                + "]" * 2_000
                + "}",
                encoding="utf-8",
            )
            result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_mutation_during_production_read_is_incompatible_not_healthy(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            target = state.marketplace / "plugins/codexy/hooks/hooks.json"
            contents = target.read_text(encoding="utf-8")
            original_read = os.read

            def mutate(descriptor: int, size: int) -> bytes:
                target.write_text(contents, encoding="utf-8")
                return original_read(descriptor, size)

            with patch("codexy_runtime_tools.component_source_admission.os.read", side_effect=mutate):
                result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(_health(result), _incompatible("core"))

    def test_windows_reparse_ancestry_change_during_read_is_incompatible(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugin = state.marketplace / "plugins/codexy"
            original_read, original_lstat = os.read, os.lstat
            changed = False

            def mutate(descriptor: int, size: int) -> bytes:
                nonlocal changed
                changed = True
                return original_read(descriptor, size)

            def reparse(path: str | Path):
                metadata = original_lstat(path)
                attributes = 0x0400 if changed and Path(path) == plugin / "hooks" else 0
                return SimpleNamespace(
                    st_mode=metadata.st_mode,
                    st_dev=metadata.st_dev,
                    st_ino=metadata.st_ino,
                    st_size=metadata.st_size,
                    st_mtime_ns=metadata.st_mtime_ns,
                    st_ctime_ns=metadata.st_ctime_ns,
                    st_file_attributes=attributes,
                )

            with patch("codexy_runtime_tools.component_source_admission.os.read", side_effect=mutate), patch("codexy_runtime_tools.component_source_admission.os.lstat", side_effect=reparse):
                result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertTrue(changed)
        self.assertEqual(_health(result), _incompatible("core"))


def _health(result: dict[str, object]) -> dict[str, object]:
    return result["component_health"][0]  # type: ignore[index]


def _incompatible(component: str) -> dict[str, str]:
    return {
        "component": component,
        "state": "incompatible",
        "repair": "repair the Codexy registration, then rerun getcodexy doctor",
    }


if __name__ == "__main__":
    unittest.main()
