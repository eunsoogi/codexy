"""GitHub role projections must participate in transactional file recovery."""

from __future__ import annotations

import os
import subprocess
import sys
import unittest
from pathlib import Path

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_transaction_snapshot import InventorySnapshot
from codexy_runtime_tools.updater import sync_agents
from packages.getcodexy.tests.component_lifecycle_support import fixture


CORE_MARKER = b"# CODEXY MANAGED AGENT\n"
GITHUB_MARKER = b"# Managed by Codexy GitHub.\n"


class ComponentSnapshotGitHubRoleTests(unittest.TestCase):
    @unittest.skipIf(os.name == "nt", "creating a symlink requires Windows privileges")
    def test_capture_rejects_symlinked_github_role_root(self) -> None:
        with fixture() as state:
            outside = state.root / "outside-role-root"
            outside.mkdir()
            role = outside / "personal.toml"
            role_bytes = b'name = "personal role"\n'
            role.write_bytes(role_bytes)
            github_root = state.home / "agents/codexy-github"
            github_root.parent.mkdir(parents=True)
            github_root.symlink_to(outside, target_is_directory=True)

            with self.assertRaisesRegex(ValueError, "symlink"):
                InventorySnapshot.capture(state.home)
            # Snapshot inspection must reject the link without changing its target.
            self.assertTrue(github_root.is_symlink())
            self.assertEqual(role.read_bytes(), role_bytes)

    def test_restore_recovers_core_and_github_managed_role_files(self) -> None:
        with fixture(real_registration=True) as state:
            run_operation(
                "install",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-snapshot-install",
            )
            core_root = state.home / "agents/codexy"
            github_root = state.home / "agents/codexy-github"
            github_script = (
                state.marketplace
                / "plugins/codexy-github/skills/git-workflow/scripts/bootstrap_codexy_github_agent.py"
            )
            _run_github_script(github_script, state.home)
            before = {
                "core": _managed_files(core_root, CORE_MARKER),
                "github": _managed_files(github_root, GITHUB_MARKER),
            }
            snapshot = InventorySnapshot.capture(state.home)

            self.assertEqual(
                sync_agents(
                    state.marketplace / "plugins/codexy", state.home, "uninstall"
                ).status,
                "completed",
            )
            _run_github_script(github_script, state.home, uninstall=True)

            snapshot.restore(state.home)

            self.assertEqual(_managed_files(core_root, CORE_MARKER), before["core"])
            self.assertEqual(
                _managed_files(github_root, GITHUB_MARKER), before["github"]
            )


def _run_github_script(script: Path, home: Path, *, uninstall: bool = False) -> None:
    command = [sys.executable, "-B", str(script), "--codex-home", str(home)]
    if uninstall:
        command.append("--uninstall")
    result = subprocess.run(
        command,
        capture_output=True,
        text=True,
        check=False,
        env={"PYTHONNOUSERSITE": "1"},
    )
    if result.returncode != 0:
        raise AssertionError(f"GitHub role operation failed: {result.stderr}")


def _managed_files(root: Path, marker: bytes) -> dict[str, bytes]:
    return {
        path.name: path.read_bytes()
        for path in sorted(root.glob("*.toml"))
        if path.is_file()
        and not path.is_symlink()
        and path.read_bytes().startswith(marker)
    }
