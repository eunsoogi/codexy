"""Lifecycle ownership, cleanup, and recovery cases for standalone agent roles."""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_lifecycle_preflight import existing_marketplace
from codexy_runtime_tools.component_lifecycle_recovery import recover_if_needed
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_transaction_state import (
    InventorySnapshot,
    Journal,
    read_journal,
    write_journal,
)
from codexy_runtime_tools.updater import sync_agents
from packages.getcodexy.tests.component_lifecycle_records import recorded
from packages.getcodexy.tests.component_lifecycle_support import fixture


CORE_MARKER = b"# CODEXY MANAGED AGENT\n"
GITHUB_MARKER = b"# Managed by Codexy GitHub.\n"


class ComponentLifecycleRoleRemovalTests(unittest.TestCase):
    def test_install_then_remove_core_reconciles_only_managed_role_files(self) -> None:
        with fixture(real_registration=True) as state:
            installed = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-role-install",
            )
            root = state.home / "agents/codexy"
            self.assertEqual(installed["outcome"], "completed")
            self.assertIn("codexy-sentinel.toml", _managed_files(root, CORE_MARKER))
            user_file = root / "my-agent.toml"
            user_bytes = b'name = "my-agent"\n'
            user_file.write_bytes(user_bytes)

            removed = run_operation(
                "remove",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-role-remove",
            )
            mutations = tuple(state.mutations)
            repeated = run_operation(
                "remove",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-role-remove",
            )
            self.assertEqual(removed["outcome"], "completed")
            self.assertEqual(repeated, removed)
            self.assertEqual(tuple(state.mutations), mutations)
            self.assertEqual(_managed_files(root, CORE_MARKER), {})
            self.assertEqual(user_file.read_bytes(), user_bytes)
            self.assertEqual(state.selection, set())
            self.assertEqual(recorded(state.home), [])

    def test_partial_remove_cleans_github_role_and_keeps_core_projection(self) -> None:
        with fixture(real_registration=True) as state:
            installed = run_operation(
                "install",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-partial-install",
            )
            self.assertEqual(installed["selection_after"], ["core", "github"])
            _run_github_projection(state)
            core_user = state.home / "agents/codexy/my-agent.toml"
            github_user = state.home / "agents/codexy-github/my-agent.toml"
            core_user.write_bytes(b"core user role\n")
            github_user.write_bytes(b"github user role\n")
            stale_role = state.home / "agents/codexy-github/old-role.toml"
            stale_role.write_bytes(GITHUB_MARKER + b"name = 'old-weaver'\n")

            removed = run_operation(
                "remove",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-partial-remove",
            )
            self.assertEqual(removed["outcome"], "completed")
            self.assertIn(
                "codexy-sentinel.toml",
                _managed_files(state.home / "agents/codexy", CORE_MARKER),
            )
            self.assertEqual(
                _managed_files(state.home / "agents/codexy-github", GITHUB_MARKER),
                {},
            )
            self.assertFalse(stale_role.exists())
            self.assertEqual(core_user.read_bytes(), b"core user role\n")
            self.assertEqual(github_user.read_bytes(), b"github user role\n")
            self.assertEqual(state.selection, {"core"})
            self.assertEqual(recorded(state.home), ["core"])

    def test_late_github_ownership_conflict_rolls_back_removed_core_roles(self) -> None:
        with fixture(real_registration=True) as state:
            run_operation(
                "install",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-conflict-install",
            )
            _run_github_projection(state)
            core_root = state.home / "agents/codexy"
            core_before = _managed_files(core_root, CORE_MARKER)
            github_role = state.home / "agents/codexy-github/codexy-weaver.toml"
            user_bytes = b'name = "user-owned-weaver"\n'
            github_role.write_bytes(user_bytes)

            removed = run_operation(
                "remove",
                ("core", "github"),
                state.home,
                state.codex,
                state.run,
                operation_id="op-conflict-remove",
            )
            self.assertEqual(removed["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core", "github"})
            self.assertEqual(recorded(state.home), ["core", "github"])
            self.assertEqual(_managed_files(core_root, CORE_MARKER), core_before)
            self.assertEqual(github_role.read_bytes(), user_bytes)
            self.assertIsNone(read_journal(state.home))

    def test_interrupted_removal_recovery_finishes_partial_role_cleanup(self) -> None:
        with fixture(real_registration=True) as state:
            run_operation(
                "install",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-interrupted-install",
            )
            _run_github_projection(state)
            snapshot = InventorySnapshot.capture(state.home)
            write_journal(
                state.home,
                Journal(
                    "op-interrupted-remove",
                    "remove",
                    ("core", "github"),
                    ("core", "github"),
                    ("core", "github"),
                    (),
                    snapshot,
                    "started",
                ),
            )
            state.selection.clear()
            core_root = state.home / "agents/codexy"
            core_roles = _managed_files(core_root, CORE_MARKER)
            (core_root / sorted(core_roles)[0]).unlink()
            manifest = load_component_manifest()
            root = existing_marketplace(state.codex, state.run, manifest)
            self.assertIsNotNone(root)

            recover_if_needed(state.home, state.codex, state.run, manifest, root)
            self.assertEqual(_managed_files(core_root, CORE_MARKER), {})
            self.assertEqual(
                _managed_files(state.home / "agents/codexy-github", GITHUB_MARKER),
                {},
            )
            self.assertEqual(state.selection, set())
            self.assertEqual(recorded(state.home), [])
            self.assertIsNone(read_journal(state.home))


def _run_github_projection(state) -> None:
    script = (
        state.marketplace
        / "plugins/codexy-github/skills/git-workflow/scripts/bootstrap_codexy_github_agent.py"
    )
    result = subprocess.run(
        [sys.executable, "-B", str(script), "--codex-home", str(state.home)],
        capture_output=True,
        text=True,
        check=False,
        env={"PYTHONNOUSERSITE": "1"},
    )
    if result.returncode != 0:
        raise AssertionError(f"GitHub role setup failed: {result.stderr}")


def _managed_files(root: Path, marker: bytes) -> dict[str, bytes]:
    if not root.is_dir() or root.is_symlink():
        return {}
    return {
        path.name: path.read_bytes()
        for path in sorted(root.glob("*.toml"))
        if path.is_file()
        and not path.is_symlink()
        and path.read_bytes().startswith(marker)
    }
