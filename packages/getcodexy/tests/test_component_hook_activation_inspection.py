from __future__ import annotations

import json
import shutil
import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.component_inspection import doctor, status
from packages.getcodexy.tests.capability_probe_cases import materialize
from packages.getcodexy.tests.component_lifecycle_support import fixture


class ComponentHookActivationInspectionTests(unittest.TestCase):
    def test_doctor_reports_pending_trust_when_a_new_required_hook_is_not_active(
        self,
    ) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugin = state.marketplace / "plugins/codexy"
            rows = [
                row
                for row in _hook_rows(plugin)
                if any(
                    f":{event}:0:" in row["key"]
                    for event in ("pre_tool_use", "permission_request")
                )
            ]
            result = doctor(
                state.home,
                codex=state.codex,
                runner=state.run,
                hook_lister=lambda _executable, _cwd: rows,
            )

        health = result["component_health"][0]
        self.assertEqual(health["state"], "pending-trust")
        self.assertEqual(health["first_failure_stage"], "activation")
        self.assertEqual(health["reason_code"], "required-hook-trust-missing")
        self.assertEqual(result["errors"], [{"code": "required-hook-trust-missing"}])

    def test_status_and_doctor_detect_missing_disabled_and_stale_hook_entries(
        self,
    ) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugin = state.marketplace / "plugins/codexy"
            rows = _hook_rows(plugin)
            cases = (
                (
                    "missing",
                    lambda current: [
                        row
                        for row in current
                        if row["key"]
                        != "codexy@codexy:hooks/hooks.json:pre_tool_use:1:0"
                    ],
                    "required-hook-trust-missing",
                ),
                (
                    "disabled",
                    lambda current: [
                        {
                            **row,
                            "enabled": False
                            if row["key"]
                            == "codexy@codexy:hooks/hooks.json:pre_tool_use:1:0"
                            else row["enabled"],
                        }
                        for row in current
                    ],
                    "required-hook-disabled",
                ),
                (
                    "stale",
                    lambda current: [
                        {
                            **row,
                            "trustStatus": "modified"
                            if row["key"]
                            == "codexy@codexy:hooks/hooks.json:pre_tool_use:1:0"
                            else row["trustStatus"],
                        }
                        for row in current
                    ],
                    "required-hook-trust-stale",
                ),
            )
            for name, mutate, reason in cases:
                with self.subTest(case=name):
                    observed = mutate([dict(row) for row in rows])
                    status_result = status(
                        state.home,
                        codex=state.codex,
                        runner=state.run,
                        hook_lister=lambda _executable, _home: observed,
                    )
                    doctor_result = doctor(
                        state.home,
                        codex=state.codex,
                        runner=state.run,
                        hook_lister=lambda _executable, _home: observed,
                    )
                    self.assertEqual(status_result["errors"], [{"code": reason}])
                    self.assertEqual(doctor_result["errors"], [{"code": reason}])
                    self.assertEqual(
                        doctor_result["component_health"][0]["reason_code"], reason
                    )

    def test_status_rejects_a_hook_from_an_alternate_plugin_root(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugin = state.marketplace / "plugins/codexy"
            alternate = state.root / "alternate/plugins/codexy"
            shutil.copytree(plugin, alternate)
            rows = [
                {
                    **row,
                    "command": row["command"].replace(str(plugin), str(alternate)),
                    "sourcePath": str(alternate / "hooks/hooks.json"),
                }
                for row in _hook_rows(plugin)
            ]
            result = status(
                state.home,
                codex=state.codex,
                runner=state.run,
                hook_lister=lambda _executable, _home: rows,
            )

        self.assertEqual(result["errors"], [{"code": "required-hook-trust-stale"}])

    def test_status_accepts_the_host_cache_copy_of_installed_registration(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            plugin = state.marketplace / "plugins/codexy"
            cache = state.home / "plugins/cache/codexy/codexy/1.6.3"
            shutil.copytree(plugin, cache)
            rows = [
                {
                    **row,
                    "command": row["command"].replace(str(plugin), str(cache)),
                    "sourcePath": str(cache / "hooks/hooks.json"),
                }
                for row in _hook_rows(plugin)
            ]
            result = status(
                state.home,
                codex=state.codex,
                runner=state.run,
                hook_lister=lambda _executable, _home: rows,
            )

        self.assertEqual(result["errors"], [])

    def test_doctor_does_not_execute_an_untrusted_hook_launcher(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            with patch(
                "codexy_runtime_tools.component_health._probe_component",
                side_effect=AssertionError("untrusted hook launcher was executed"),
            ):
                result = doctor(
                    state.home,
                    codex=state.codex,
                    runner=state.run,
                    hook_lister=lambda _executable, _home: [],
                )
        self.assertEqual(result["component_health"][0]["state"], "pending-trust")
        self.assertEqual(
            result["component_health"][0]["reason_code"],
            "required-hook-trust-missing",
        )


def _hook_rows(plugin: Path) -> list[dict[str, object]]:
    events = {
        "PreToolUse": "preToolUse",
        "PermissionRequest": "permissionRequest",
        "UserPromptSubmit": "userPromptSubmit",
    }
    event_keys = {
        "PreToolUse": "pre_tool_use",
        "PermissionRequest": "permission_request",
        "UserPromptSubmit": "user_prompt_submit",
    }
    value = json.loads((plugin / "hooks/hooks.json").read_text(encoding="utf-8"))
    path = plugin / "hooks/hooks.json"
    rows = []
    for event, groups in value["hooks"].items():
        for group_index, group in enumerate(groups):
            for hook_index, hook in enumerate(group["hooks"]):
                command = hook["command"].replace("${PLUGIN_ROOT}", str(plugin))
                rows.append(
                    {
                        "key": f"codexy@codexy:hooks/hooks.json:{event_keys[event]}:{group_index}:{hook_index}",
                        "eventName": events[event],
                        "handlerType": "command",
                        "command": command,
                        "async": hook.get("async", False),
                        "matcher": group.get("matcher"),
                        "timeoutSec": hook.get("timeout", 600),
                        "sourcePath": str(path),
                        "pluginId": "codexy@codexy",
                        "enabled": True,
                        "isManaged": False,
                        "currentHash": "sha256:fixture",
                        "trustStatus": "trusted",
                    }
                )
    return rows


if __name__ == "__main__":
    unittest.main()
