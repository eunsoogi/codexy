"""Cache-repair assertions kept behind the CI-selected lifecycle watcher test."""

import shutil
from unittest import TestCase
from unittest.mock import patch

from codexy_runtime_tools import (
    component_lifecycle_recovery,
    component_registration_health,
)
from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_mcp_materialization import (
    component_cache_plugin,
    materialize_component_mcp,
)
from packages.getcodexy.tests.component_hook_registration_fixture import (
    completed_registration,
    fixture_hook_rows,
)
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import VERSION, fixture


def assert_update_repairs_a_stale_host_cache_surface(test_case: TestCase) -> None:
    with fixture({"core"}) as state:
        record(state.home, ["core"])
        source_plugin = state.marketplace / "plugins/codexy"
        materialize_component_mcp(source_plugin, "core", VERSION)
        cache = component_cache_plugin(state.home, "core", VERSION)
        shutil.copytree(source_plugin, cache)
        cache_bootstrap = cache / "mcp/codexy_mcp_bootstrap.py"
        # Model a prior registered package left in the selected host cache.
        cache_bootstrap.parent.mkdir(parents=True)
        # No-MCP repair preserves dormant bytes; write bytes to avoid Windows CRLF.
        cache_bootstrap.write_bytes(b"stale cache surface\n")
        (cache / ".mcp.json").write_text('{"watcher": {}}\n', encoding="utf-8")

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
        ):
            receipt = run_operation(
                "update",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-update-cache-mcp",
                hook_lister=lambda _executable, _home: fixture_hook_rows(
                    state.marketplace
                ),
            )

        test_case.assertEqual(
            receipt["outcome"],
            "completed",
            {
                "receipt": receipt,
                "mutations": state.mutations,
            },
        )
        test_case.assertFalse((cache / ".mcp.json").exists())
        test_case.assertEqual(
            (cache / ".codex-plugin/mcp-capability.json").read_bytes(),
            (source_plugin / ".codex-plugin/mcp-capability.json").read_bytes(),
        )
        test_case.assertEqual(cache_bootstrap.read_bytes(), b"stale cache surface\n")
