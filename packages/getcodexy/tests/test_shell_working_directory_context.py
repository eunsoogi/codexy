"""Regression coverage for known and conflicting shell working-directory state."""

from __future__ import annotations

import importlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path
from typing import cast

_support = importlib.import_module("github_native_hook_support")
PLUGIN = cast(Path, getattr(_support, "PLUGIN"))
native_command = cast(
    Callable[[list[str]], list[str]], getattr(_support, "native_command")
)

# Load the policy modules directly to assert the context value as well as hook admission.
sys.path.insert(0, str(PLUGIN / "hooks"))
from codexy_policy.execution_context import assign  # noqa: E402
from codexy_policy.execution_context_types import ExecutionContext  # noqa: E402
from codexy_policy.shell_context import after_cd, at  # noqa: E402
from codexy_policy.shell_entry import context as shell_context  # noqa: E402


class ShellWorkingDirectoryContextTests(unittest.TestCase):
    def _context(self, cwd: str) -> ExecutionContext:
        return shell_context(
            cwd,
            None,
            None,
            None,
            (),
            (
                ("HOME", "/tmp"),
                ("PATH", "/usr/bin"),
                ("USER", "fixture"),
                ("PWD", "/hook/runner/directory"),
            ),
        )

    def _classify(
        self, command: str, event: str = "PreToolUse", cwd: str | None = None
    ) -> str:
        hook = PLUGIN / "hooks/codexy-destructive-command.sh"
        cwd = cwd or str(PLUGIN.parents[1])
        payload = json.dumps(
            {
                "hook_event_name": event,
                "tool_name": "Bash",
                "tool_input": {"command": command},
                "cwd": cwd,
            }
        )
        # This sends inert command text to the policy hook; it never runs the shell command.
        result = subprocess.run(
            native_command([str(hook), event]),
            input=payload,
            text=True,
            capture_output=True,
            env={**os.environ, "PLUGIN_ROOT": str(PLUGIN)},
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_request_cwd_supplies_known_pwd_for_assignment_expansion(self) -> None:
        cwd = str(PLUGIN.parents[1])
        context = self._context(cwd)

        assigned = assign("worktree_path=$PWD", context)

        self.assertEqual(dict(assigned.environment)["PWD"], cwd)
        self.assertEqual(dict(assigned.environment)["worktree_path"], cwd)
        self.assertFalse(assigned.opaque_environment)

    def test_relative_request_cwd_does_not_supply_pwd(self) -> None:
        context = self._context("relative/directory")

        assigned = assign("worktree_path=$PWD", context)
        conflicting = assign("PWD=relative/directory", context)

        self.assertNotIn("PWD", dict(context.environment))
        self.assertTrue(assigned.opaque_environment)
        self.assertTrue(conflicting.opaque_environment)

    def test_scoped_directory_does_not_rewrite_shell_pwd(self) -> None:
        cwd = str(PLUGIN.parents[1])
        plugins = str(Path(cwd) / "plugins")
        context = self._context(cwd)

        shell_context_after_cd = after_cd(context, plugins)
        scoped_context = at(context, plugins)

        self.assertEqual(dict(shell_context_after_cd.environment)["PWD"], plugins)
        self.assertEqual(scoped_context.cwd, plugins)
        self.assertEqual(dict(scoped_context.environment)["PWD"], cwd)

    def test_known_pwd_allows_bounded_read_only_git_status(self) -> None:
        command = 'worktree_path="$PWD" git status --short'
        for event in ("PermissionRequest", "PreToolUse"):
            with self.subTest(event=event):
                self.assertEqual(self._classify(command, event), "")

    def test_shell_cd_keeps_pwd_assignment_in_the_current_directory(self) -> None:
        command = 'cd plugins && PWD="$PWD" git status --short'
        for event in ("PermissionRequest", "PreToolUse"):
            with self.subTest(event=event):
                self.assertEqual(self._classify(command, event), "")

    def test_shell_cd_does_not_let_stale_pwd_hide_repository_deletion(self) -> None:
        repository = str(PLUGIN.parents[1])
        command = (
            'initial_directory="$PWD"\n'
            f'cd "{repository}" && cd "$PWD" && rm -rf packages'
        )
        for event in ("PermissionRequest", "PreToolUse"):
            with self.subTest(event=event):
                output = self._classify(command, event, cwd="/tmp")
                self.assertIn("CODEXY_DESTRUCTIVE_COMMAND_DESTRUCTIVE_EFFECT", output)

    def test_failed_shell_cd_keeps_the_original_pwd_for_later_commands(self) -> None:
        repository = str(PLUGIN.parents[1])
        with tempfile.TemporaryDirectory() as temporary_directory:
            missing_directory = Path(temporary_directory) / "missing"
            command = (
                f'cd "{missing_directory}" ; cd "$PWD" '
                "&& rm -rf packages"
            )

            for event in ("PermissionRequest", "PreToolUse"):
                with self.subTest(event=event):
                    output = self._classify(command, event, cwd=repository)
                    self.assertIn(
                        "CODEXY_DESTRUCTIVE_COMMAND_DESTRUCTIVE_EFFECT", output
                    )

    def test_unknown_dynamic_and_conflicting_pwd_remain_denied(self) -> None:
        commands = (
            'PWD="$UNKNOWN_WORKING_DIRECTORY" git status --short',
            'PWD="$(pwd)" git status --short',
            "PWD=/different/directory git status --short",
        )
        for event in ("PermissionRequest", "PreToolUse"):
            for command in commands:
                with self.subTest(event=event, command=command):
                    output = json.loads(self._classify(command, event))
                    decision = (
                        output["hookSpecificOutput"]["decision"]["behavior"]
                        if event == "PermissionRequest"
                        else output["hookSpecificOutput"]["permissionDecision"]
                    )
                    self.assertEqual(decision, "deny")

    def test_known_pwd_does_not_allow_destructive_git_reset(self) -> None:
        command = 'PWD="$PWD" git reset --hard HEAD'
        for event in ("PermissionRequest", "PreToolUse"):
            with self.subTest(event=event):
                output = self._classify(command, event)
                response = json.loads(output)["hookSpecificOutput"]
                decision = (
                    response["permissionDecision"]
                    if event == "PreToolUse"
                    else response["decision"]["behavior"]
                )
                self.assertEqual(decision, "deny")
                self.assertIn("CODEXY_DESTRUCTIVE_COMMAND_DESTRUCTIVE_EFFECT", output)
