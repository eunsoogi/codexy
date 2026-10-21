"""Reject package markers that misstate the intentional no-MCP contract."""

import json
import unittest

from codexy_runtime_tools.component_mcp_materialization import (
    materialize_component_mcp,
    valid_component_mcp,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture
from packages.getcodexy.tests.component_marketplace_fixture import (
    configure_no_mcp_core_package,
)


class McpCapabilityTests(unittest.TestCase):
    def test_no_mcp_capability_rejects_a_registered_server(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            version = "2.0.0"
            configure_no_mcp_core_package(plugin, version)
            capability_path = plugin / ".codex-plugin/mcp-capability.json"
            capability = json.loads(capability_path.read_text(encoding="utf-8"))
            capability["servers"] = ["watcher"]
            capability_path.write_text(json.dumps(capability), encoding="utf-8")

            self.assertFalse(valid_component_mcp(plugin, "core", version))
            with self.assertRaisesRegex(ValueError, "capability contract is invalid"):
                materialize_component_mcp(plugin, "core", version)


if __name__ == "__main__":
    unittest.main()
