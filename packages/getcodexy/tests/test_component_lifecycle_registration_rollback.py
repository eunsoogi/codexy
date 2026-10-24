"""Late role cleanup failures restore both owned projections and user files."""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools import component_lifecycle_recovery
from codexy_runtime_tools.component_lifecycle import run_operation
from packages.getcodexy.tests.component_lifecycle_records import recorded
from packages.getcodexy.tests.component_lifecycle_support import fixture


CORE_MARKER = b"# CODEXY MANAGED AGENT\n"
GITHUB_MARKER = b"# Managed by Codexy GitHub.\n"


class ComponentLifecycleRegistrationRollbackTests(unittest.TestCase):
    def test_failure_after_both_uninstallers_restores_transaction_snapshot(
        self,
    ) -> None:
        with fixture(real_registration=True) as state:
            run_operation(
                "install",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-late-rollback-install",
            )
            _run_github_projection(state)
            core_root = state.home / "agents/codexy"
            github_root = state.home / "agents/codexy-github"
            github_role = github_root / "codexy-weaver.toml"
            # Normalize native newlines first so this CRLF fixture is stable on Windows.
            github_bytes = (
                github_role.read_bytes().replace(b"\r\n", b"\n").replace(b"\n", b"\r\n")
            )
            github_role.write_bytes(github_bytes)
            self.assertTrue(github_bytes.startswith(b"# Managed by Codexy GitHub.\r\n"))
            core_before = _managed_files(core_root, CORE_MARKER)
            github_before = _managed_files(github_root, GITHUB_MARKER)
            core_user = core_root / "my-agent.toml"
            github_user = github_root / "my-agent.toml"
            core_user.write_bytes(b"core user role\n")
            github_user.write_bytes(b"github user role\n")
            original = component_lifecycle_recovery.synchronize_component_registrations

            def uninstall_then_fail(home, root, journal):
                original(home, root, journal)
                raise RuntimeError("forced failure after role cleanup")

            with patch.object(
                component_lifecycle_recovery,
                "synchronize_component_registrations",
                side_effect=uninstall_then_fail,
            ):
                result = run_operation(
                    "remove",
                    ("core", "github"),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-late-rollback-remove",
                )

            self.assertEqual(result["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core", "github"})
            self.assertEqual(recorded(state.home), ["core", "github"])
            self.assertEqual(_managed_files(core_root, CORE_MARKER), core_before)
            self.assertEqual(_managed_files(github_root, GITHUB_MARKER), github_before)
            self.assertEqual(github_role.read_bytes(), github_bytes)
            self.assertEqual(core_user.read_bytes(), b"core user role\n")
            self.assertEqual(github_user.read_bytes(), b"github user role\n")


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
    return {
        path.name: path.read_bytes()
        for path in sorted(root.glob("*.toml"))
        if path.is_file()
        and not path.is_symlink()
        and path.read_bytes().startswith(marker)
    }
