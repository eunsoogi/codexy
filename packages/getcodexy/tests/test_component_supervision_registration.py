"""Validate the installed core hook topology including supervision."""

import json
import unittest

from codexy_runtime_tools.component_mcp_materialization import materialize_component_mcp
from codexy_runtime_tools.component_registration_health import valid_registration
from packages.getcodexy.tests.component_lifecycle_support import VERSION, fixture


class SupervisionRegistrationTests(unittest.TestCase):
    def test_core_registration_contains_routing_and_cleanup_hooks(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            materialize_component_mcp(plugin, "core", VERSION)
            hooks = json.loads((plugin / "hooks/hooks.json").read_text())["hooks"]

            self.assertEqual(
                set(hooks),
                {"PermissionRequest", "PreToolUse", "PostToolUse", "Stop", "Interrupt"},
            )
            expected_matchers = {
                "^(?:codex_app__|mcp__codex_app__)send_message_to_thread$",
                "^(?:codex_app__|mcp__codex_app__)create_thread$",
                "^(?:(?:agents|multi_agent_v1)__|agents|collaboration|multi_agent_v1)?(?:spawn_agent|send_message|followup_task|send_input)$",
            }
            for event in ("PermissionRequest", "PreToolUse"):
                self.assertEqual(
                    {group["matcher"] for group in hooks[event]}, expected_matchers
                )
            self.assertEqual(
                hooks["PostToolUse"][1]["matcher"],
                "^(?:codex_app__|mcp__codex_app__)(?:set_thread_archived|list_archived_threads)$",
            )
            self.assertEqual(
                hooks["PostToolUse"][0]["matcher"],
                "^(?:codex_app__|mcp__codex_app__)create_thread$",
            )
            for event in ("Stop", "Interrupt"):
                self.assertNotIn("matcher", hooks[event][0])
            self.assertTrue(valid_registration(plugin, "core"))
