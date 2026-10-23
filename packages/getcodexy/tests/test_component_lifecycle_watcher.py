from __future__ import annotations

import json
import os
import shutil
import unittest
from dataclasses import replace
from unittest.mock import patch

from codexy_runtime_tools import component_mcp_cache
from codexy_runtime_tools import component_registration_health
from codexy_runtime_tools import component_lifecycle_recovery
from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_lifecycle_mcp import (
    materialize_mcp_caches,
    materialize_mcp_sources,
)
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_mcp_materialization import (
    component_cache_plugin,
    materialize_component_mcp,
    valid_component_mcp_cache,
)
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import VERSION, fixture
from packages.getcodexy.tests.component_marketplace_fixture import (
    configure_no_mcp_core_package,
)
from packages.getcodexy.tests.component_lifecycle_watcher_cache_support import (
    assert_update_repairs_a_stale_host_cache_surface,
)
from packages.getcodexy.tests.component_hook_registration_fixture import (
    completed_registration,
    fixture_hook_rows,
)


class LifecycleWatcherTests(unittest.TestCase):
    def test_install_validates_each_selected_mcp_source_before_host_add(self) -> None:
        with fixture() as state:
            calls: list[tuple[str, str, tuple[tuple[str, ...], ...]]] = []
            registrations: list[dict[str, object]] = []

            def observe_registration(plugin, home, mode):
                result = completed_registration(plugin, home, mode)
                registrations.append(result.as_dict())
                return result

            def validate_source(plugin, component, version):
                # Snapshot host mutations here to prove source validation precedes add.
                calls.append((component, version, tuple(state.mutations)))
                return materialize_component_mcp(plugin, component, version)

            with (
                patch.object(
                    component_registration_health,
                    "sync_agents",
                    side_effect=completed_registration,
                ),
                patch(
                    "codexy_runtime_tools.component_lifecycle_mcp.materialize_component_mcp",
                    side_effect=validate_source,
                ),
            ):
                receipt = run_operation(
                    "install",
                    (),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-install-mcp",
                    hook_lister=lambda _executable, _home: fixture_hook_rows(
                        state.marketplace
                    ),
                )

            self.assertEqual(
                receipt["outcome"],
                "completed",
                {"receipt": receipt, "registrations": registrations, "calls": calls},
            )
            self.assertEqual(
                [(component, version) for component, version, _ in calls],
                [("core", VERSION), ("devtools", VERSION)],
            )
            self.assertTrue(
                all(
                    not any(mutation[:2] == ("plugin", "add") for mutation in snapshot)
                    for _, _, snapshot in calls
                )
            )
            core = state.marketplace / "plugins/codexy"
            self.assertFalse((core / "mcp/codexy_mcp_bootstrap.py").exists())
            self.assertFalse((core / "mcp/codexy-mcp-watcher.sh").exists())
            self.assertFalse((core / "mcp/codexy-mcp-watcher.cmd").exists())
            self.assertFalse((core / "mcp/codexy-mcp-watcher").exists())

    def test_update_validates_the_core_source_before_host_update(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core"])
            calls: list[tuple[str, str, tuple[tuple[str, ...], ...]]] = []

            def validate_source(plugin, component, version):
                # The recorded mutation prefix must still exclude the host update.
                calls.append((component, version, tuple(state.mutations)))
                return materialize_component_mcp(plugin, component, version)

            with (
                patch.object(
                    component_lifecycle_recovery,
                    "reconcile_official_marketplace_root",
                    return_value=state.marketplace,
                ),
                patch.object(
                    component_registration_health,
                    "sync_agents",
                    side_effect=completed_registration,
                ),
                patch(
                    "codexy_runtime_tools.component_lifecycle_mcp.materialize_component_mcp",
                    side_effect=validate_source,
                ),
            ):
                receipt = run_operation(
                    "update",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-update-mcp",
                    hook_lister=lambda _executable, _home: fixture_hook_rows(
                        state.marketplace
                    ),
                )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(
                [(component, version) for component, version, _ in calls],
                [("core", VERSION)],
            )
            self.assertTrue(
                all(
                    not any(mutation[:2] == ("plugin", "add") for mutation in snapshot)
                    for _, _, snapshot in calls
                )
            )

    def test_update_repairs_a_stale_host_cache_surface(self) -> None:
        assert_update_repairs_a_stale_host_cache_surface(self)

    def test_install_rolls_back_when_selected_host_cache_is_missing(self) -> None:
        with fixture() as state:
            (state.home / "plugins/cache").mkdir(parents=True)

            receipt = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-install-missing-mcp-cache",
            )

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertNotIn("core", state.selection)
            self.assertFalse(
                (state.home / "plugins/cache/codexy/codexy" / VERSION).exists()
            )

    def test_no_mcp_lifecycle_keeps_source_and_cache_surfaces_empty(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            version = "2.0.0"
            configure_no_mcp_core_package(plugin, version)
            current = load_component_manifest()
            manifest = replace(
                current,
                version=version,
                components=tuple(
                    replace(c, version=version) for c in current.components
                ),
            )

            materialize_mcp_sources(state.marketplace, manifest, ("core",))
            cache = component_cache_plugin(state.home, "core", version)
            cache.parent.mkdir(parents=True)
            shutil.copytree(plugin, cache)
            cache_args = state.home, state.marketplace, manifest, ("core",)
            with patch.object(component_mcp_cache, "_atomic_copy") as copy_file:
                materialize_mcp_caches(*cache_args)
                materialize_mcp_caches(*cache_args)

            copy_file.assert_not_called()
            self.assertTrue(
                all(not os.path.lexists(root / ".mcp.json") for root in (plugin, cache))
            )
            for root in (plugin, cache):
                for retired in (
                    "mcp/codexy_mcp_bootstrap.py",
                    "mcp/codexy-mcp-watcher.sh",
                    "mcp/codexy-mcp-watcher.cmd",
                ):
                    self.assertFalse((root / retired).exists(), retired)
            residual = cache / ".mcp.json"
            residual.write_text("{}", encoding="utf-8")
            self.assertFalse(valid_component_mcp_cache(state.home, "core", version))


if __name__ == "__main__":
    unittest.main()
