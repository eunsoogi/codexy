"""Hook regressions for bounded local output redirection."""

from __future__ import annotations

import importlib
import json
import os
import shlex
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path
from typing import cast
from unittest import mock
from uuid import uuid4

_support = importlib.import_module("github_native_hook_support")
PLUGIN = cast(Path, getattr(_support, "PLUGIN"))
native_command = cast(
    Callable[[list[str]], list[str]], getattr(_support, "native_command")
)
sys.path.insert(0, str(PLUGIN / "hooks"))
_filesystem = importlib.import_module("codexy_policy.execution_filesystem")
safe_output_redirection = cast(
    Callable[[str, str], bool], getattr(_filesystem, "safe_output_redirection")
)


class ShellOutputRedirectionTests(unittest.TestCase):
    def _classify(self, command: str, event: str) -> str:
        hook = PLUGIN / "hooks/codexy-destructive-command.sh"
        payload = json.dumps(
            {
                "hook_event_name": event,
                "tool_name": "Bash",
                "tool_input": {"command": command},
                "cwd": str(PLUGIN.parents[1]),
            }
        )
        # This runs the hook with inert command text; the requested shell command never runs.
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

    def _assert_denied(self, command: str, event: str) -> None:
        output = self._classify(command, event)
        self.assertIn("deny", output, command)
        self.assertIn("CODEXY_DESTRUCTIVE_COMMAND_", output, command)

    def test_direct_private_temp_logs_are_allowed_for_both_events(self) -> None:
        log = f"/private/tmp/codexy-issue-1248-{uuid4().hex}.log"
        commands = (
            f"gh run watch 37218256143 --repo eunsoogi/codexy > {log}",
            " ".join(
                (
                    "gh run view 111538535524 --repo eunsoogi/codexy --log >",
                    log,
                    "2>&1 && chmod 0600",
                    log,
                    "&& grep -n 'completed'",
                    log,
                )
            ),
        )
        for event in ("PermissionRequest", "PreToolUse"):
            for command in commands:
                with self.subTest(event=event, command=command):
                    self.assertEqual(self._classify(command, event), "", command)

    def test_prior_link_commands_cannot_prepare_a_temp_redirection(self) -> None:
        source = shlex.quote(str(PLUGIN.parents[1] / "README.md"))
        private_temp = Path("/private") / "tmp"
        for link in ("ln -s", "ln"):
            target = f"/private/tmp/codexy-issue-1248-{uuid4().hex}.log"
            substitution = f'"$({link} {source} {target})"'
            commands = [
                f"gh run view 1 --repo eunsoogi/codexy --log {substitution} > {target}"
            ]
            # Only model a planted link where this host admits and can open a temp target.
            if private_temp.is_dir() and safe_output_redirection(">", target):
                setup = f"{link} {source} {target}"
                commands.append(
                    f"{setup} && gh run view 1 --repo eunsoogi/codexy --log > {target}"
                )
            for event in ("PermissionRequest", "PreToolUse"):
                for command in commands:
                    with self.subTest(event=event, link=link, command=command):
                        self._assert_denied(command, event)

    def test_nested_shell_inherits_substitution_and_prior_link_restrictions(
        self,
    ) -> None:
        directory = Path("/private") / "tmp"
        if not directory.is_dir():
            self.skipTest("private temp redirection requires a Unix host")
        source = shlex.quote(str(PLUGIN.parents[1] / "README.md"))
        for link in ("ln -s", "ln"):
            target = f"/private/tmp/codexy-issue-1248-{uuid4().hex}.log"
            if not safe_output_redirection(">", target):
                self.skipTest("private temp output is not valid on this host")
            script = shlex.quote(
                f"gh run view 1 --repo eunsoogi/codexy --log > {target}"
            )
            commands = (
                f"sh -c {script}",
                f'sh -c {script} "$({link} {source} {target})"',
                f"{link} {source} {target} && sh -c {script}",
            )
            for event in ("PermissionRequest", "PreToolUse"):
                with self.subTest(event=event, link=link, command=commands[0]):
                    self.assertEqual(self._classify(commands[0], event), "")
                for command in commands[1:]:
                    with self.subTest(event=event, link=link, command=command):
                        self._assert_denied(command, event)

    def test_quoted_dollar_path_marker_is_never_checked_as_a_different_file(
        self,
    ) -> None:
        target = f"/private/tmp/$codexy-issue-1248-{uuid4().hex}.log"
        command = f"gh run view 1 --repo eunsoogi/codexy --log > {shlex.quote(target)}"
        for event in ("PermissionRequest", "PreToolUse"):
            with self.subTest(event=event):
                self._assert_denied(command, event)

    def test_quoted_angle_path_markers_cannot_hide_existing_symlinks(self) -> None:
        directory = Path("/private") / "tmp"
        if not directory.is_dir():
            self.skipTest("private temp marker targets require a Unix host")
        for marker in "<>":
            target = directory / f"{marker}codexy-issue-1248-{uuid4().hex}.log"
            _ = target.symlink_to(PLUGIN.parents[1] / "README.md")
            try:
                command = (
                    "gh run view 1 --repo eunsoogi/codexy --log > "
                    f"{shlex.quote(str(target))}"
                )
                for event in ("PermissionRequest", "PreToolUse"):
                    with self.subTest(event=event, marker=marker):
                        self._assert_denied(command, event)
            finally:
                target.unlink(missing_ok=True)

    def test_dev_null_stays_safe_after_a_prior_shell_segment(self) -> None:
        for event in ("PermissionRequest", "PreToolUse"):
            with self.subTest(event=event):
                self.assertEqual(
                    self._classify("true && gh repo view > /dev/null", event), ""
                )

    def test_existing_dollar_named_symlink_is_denied_for_both_events(self) -> None:
        directory = Path("/private") / "tmp"
        if not directory.is_dir():
            self.skipTest("macOS private temp directory is unavailable")
        target = directory / f"$codexy-issue-1248-{uuid4().hex}.log"
        _ = target.symlink_to(PLUGIN.parents[1] / "README.md")
        try:
            command = (
                "gh run view 1 --repo eunsoogi/codexy --log > "
                f"{shlex.quote(str(target))}"
            )
            for event in ("PermissionRequest", "PreToolUse"):
                with self.subTest(event=event):
                    self._assert_denied(command, event)
        finally:
            target.unlink(missing_ok=True)

    def test_existing_symlink_target_is_denied_for_both_events(self) -> None:
        directory = Path("/private") / "tmp"
        if not directory.is_dir():
            self.skipTest("macOS private temp directory is unavailable")
        target = directory / f"codexy-issue-1248-{uuid4().hex}.log"
        _ = target.symlink_to(PLUGIN.parents[1] / "README.md")
        try:
            command = f"gh run view 1 --repo eunsoogi/codexy --log > {target}"
            for event in ("PermissionRequest", "PreToolUse"):
                with self.subTest(event=event):
                    self._assert_denied(command, event)
        finally:
            target.unlink(missing_ok=True)

    def test_existing_hard_link_target_is_denied_for_both_events(self) -> None:
        directory = Path("/private") / "tmp"
        if not directory.is_dir():
            self.skipTest("macOS private temp directory is unavailable")
        target = directory / f"codexy-issue-1248-{uuid4().hex}.log"
        try:
            os.link(PLUGIN.parents[1] / "README.md", target)
        except OSError as error:
            self.skipTest(f"hard links unavailable for this workspace: {error}")
        try:
            command = f"gh run view 1 --repo eunsoogi/codexy --log > {target}"
            for event in ("PermissionRequest", "PreToolUse"):
                with self.subTest(event=event):
                    self._assert_denied(command, event)
        finally:
            target.unlink(missing_ok=True)

    def test_private_temp_output_requires_current_user_ownership(self) -> None:
        directory = Path("/private") / "tmp"
        owner = getattr(os, "geteuid", None)
        if not directory.is_dir() or owner is None:
            self.skipTest("private temp ownership checks require a Unix host")
        with tempfile.NamedTemporaryFile(
            dir=directory, prefix="codexy-issue-1248-", delete=False
        ) as output:
            target = Path(output.name)
        try:
            self.assertTrue(safe_output_redirection(">", str(target)))
            with mock.patch(
                "codexy_policy.execution_filesystem.os.geteuid",
                return_value=owner() + 1,
            ):
                self.assertFalse(safe_output_redirection(">", str(target)))
        finally:
            target.unlink(missing_ok=True)


if __name__ == "__main__":
    _ = unittest.main()
