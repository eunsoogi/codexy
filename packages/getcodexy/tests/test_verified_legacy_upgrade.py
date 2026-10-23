"""Cover the verified legacy package as the source of a real update lifecycle."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import unittest
from collections.abc import Callable
from pathlib import Path
from unittest.mock import patch

from core_hook_test_support import temporary_primary_checkout, worker_creation_input
from codexy_runtime_tools import component_lifecycle_recovery
from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.updater import compare_managed_files
from packages.getcodexy.tests.component_hook_registration_fixture import (
    fixture_hook_rows,
)
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import fixture
from packages.getcodexy.tests.component_marketplace_fixture import (
    make_no_mcp_core_package,
    populate_plugins,
)
from packages.getcodexy.tests.component_watcher_fixture import (
    verified_legacy_marketplace_root,
)


def _run_candidate_worker_creation_hook(
    plugin: Path, primary_checkout: Path, tool_input: dict[str, object]
) -> subprocess.CompletedProcess[bytes]:
    """Execute the updated package's native launcher with a real hook envelope."""
    suffix = "cmd" if os.name == "nt" else "sh"
    launcher = plugin / "hooks" / f"codexy-child-thread-creation.{suffix}"
    command = [str(launcher), "PreToolUse"]
    if os.name == "nt":
        command = [
            os.environ.get("COMSPEC", "cmd.exe"),
            "/d",
            "/c",
            "call",
            *command,
        ]
    environment = os.environ.copy()
    environment["PLUGIN_ROOT"] = str(plugin)
    payload = {
        "cwd": str(primary_checkout),
        "hook_event_name": "PreToolUse",
        "tool_name": "mcp__codex_app__create_thread",
        "tool_input": tool_input,
    }
    return subprocess.run(
        command,
        input=json.dumps(payload).encode(),
        capture_output=True,
        env=environment,
        check=False,
    )


class VerifiedLegacyUpgradeTests(unittest.TestCase):
    def test_update_migrates_the_verified_legacy_package_to_current_core(self) -> None:
        legacy_root = verified_legacy_marketplace_root()
        if legacy_root is None:
            self.skipTest("no verified 1.13.4 public bundle is configured")

        with fixture({"core"}, real_registration=True) as state:
            # Seed the authenticated old package, then model the host's package fetch
            # only after its marketplace upgrade succeeds.
            shutil.rmtree(state.marketplace / "plugins")
            shutil.copytree(legacy_root / "plugins", state.marketplace / "plugins")
            legacy_core = state.marketplace / "plugins/codexy"
            legacy_watcher = legacy_core / "mcp/codexy-mcp-watcher.exe"
            self.assertTrue((legacy_core / ".mcp.json").is_file())
            self.assertEqual(legacy_watcher.read_bytes()[:2], b"MZ")

            candidate = state.root / "candidate-marketplace"
            populate_plugins(candidate)
            candidate_core = candidate / "plugins/codexy"
            candidate_version, candidate_manifest = make_no_mcp_core_package(
                candidate_core, "2.0.0"
            )
            original_run = state.run

            def upgrade_marketplace(
                command: list[str],
            ) -> subprocess.CompletedProcess[str]:
                result = original_run(command)
                if (
                    tuple(command[1:4]) == ("plugin", "marketplace", "upgrade")
                    and result.returncode == 0
                ):
                    shutil.rmtree(state.marketplace / "plugins")
                    shutil.copytree(
                        candidate / "plugins", state.marketplace / "plugins"
                    )
                    state.versions = {"core": candidate_version}
                elif (
                    tuple(command[1:3]) == ("plugin", "add")
                    and command[3] == "codexy@codexy"
                    and result.returncode == 0
                ):
                    state.versions["core"] = candidate_version
                return result

            def reconcile_marketplace(
                executable: Path,
                invoke: Callable[[list[str]], subprocess.CompletedProcess[str]],
                _target_version: str,
                _home: Path,
            ) -> Path:
                result = invoke(
                    [
                        str(executable),
                        "plugin",
                        "marketplace",
                        "upgrade",
                        "codexy",
                        "--json",
                    ]
                )
                if result.returncode != 0:
                    raise RuntimeError("fixture marketplace upgrade failed")
                return state.marketplace

            record(state.home, ["core"])
            with (
                patch.object(
                    component_lifecycle_recovery,
                    "reconcile_official_marketplace_root",
                    side_effect=reconcile_marketplace,
                ),
                patch(
                    "codexy_runtime_tools.component_lifecycle_operation.load_component_manifest",
                    return_value=candidate_manifest,
                ),
            ):
                receipt = run_operation(
                    "update",
                    ("core",),
                    state.home,
                    state.codex,
                    upgrade_marketplace,
                    operation_id="op-verified-legacy-upgrade",
                    hook_lister=lambda _executable, _home: fixture_hook_rows(
                        state.marketplace
                    ),
                )

            self.assertEqual(
                receipt["outcome"],
                "completed",
                {
                    "receipt": receipt,
                    "calls": state.calls,
                    "mutations": state.mutations,
                },
            )
            self.assertEqual(
                sum(
                    call[:3] == ("plugin", "marketplace", "upgrade")
                    for call in state.calls
                ),
                1,
            )
            updated_core = state.marketplace / "plugins/codexy"
            self.assertFalse((updated_core / ".mcp.json").exists())
            self.assertFalse((updated_core / "mcp/codexy-mcp-watcher.exe").exists())
            self.assertFalse(
                (
                    updated_core / "runtime/codexy-mcp-watcher-windows-x86_64.exe"
                ).exists()
            )
            self.assertEqual(
                (updated_core / "hooks/hooks.json").read_bytes(),
                (candidate_core / "hooks/hooks.json").read_bytes(),
            )
            roles = compare_managed_files(updated_core, state.home, "core")
            self.assertEqual(roles["state"], "exact", roles)

            with temporary_primary_checkout() as primary_checkout:
                allowed = _run_candidate_worker_creation_hook(
                    updated_core, primary_checkout, worker_creation_input()
                )
                self.assertEqual(
                    (allowed.returncode, allowed.stdout, allowed.stderr),
                    (0, b"", b""),
                )
                invalid_input = worker_creation_input()
                invalid_input["target"] = {"type": "local"}
                denied = _run_candidate_worker_creation_hook(
                    updated_core, primary_checkout, invalid_input
                )
                self.assertEqual((denied.returncode, denied.stderr), (0, b""))
                denial = json.loads(denied.stdout)["hookSpecificOutput"]
                self.assertEqual(denial["permissionDecision"], "deny")
                self.assertIn(
                    "APP_WORKTREE_REQUIRED",
                    denial["permissionDecisionReason"],
                )
