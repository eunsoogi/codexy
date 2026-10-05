"""Hook regressions for compiler commands, executable aliases, and log output."""

from __future__ import annotations

import importlib
import json
import os
import shlex
import shutil
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
# Load the standalone identity helper to simulate shared host launcher metadata.
sys.path.insert(0, str(PLUGIN / "hooks"))
_digest = importlib.import_module("codexy_policy.executable_digest")
same_executable = cast(
    Callable[[Path, Path], bool], getattr(_digest, "same_executable")
)
_filesystem = importlib.import_module("codexy_policy.execution_filesystem")
safe_output_redirection = cast(
    Callable[[str, str], bool], getattr(_filesystem, "safe_output_redirection")
)


class ShellExecutableIdentityTests(unittest.TestCase):
    def test_shared_launcher_identity_does_not_override_command_names(self) -> None:
        with (
            mock.patch("os.path.samefile", return_value=True),
            mock.patch("codexy_policy.executable_digest.digest", return_value=b"same"),
        ):
            self.assertFalse(
                same_executable(Path("/usr/bin/swiftc"), Path("/usr/bin/git"))
            )

    def _classify(self, command: str) -> str:
        hook = PLUGIN / "hooks/codexy-destructive-command.sh"
        payload = json.dumps(
            {
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "tool_input": {"command": command},
                "cwd": str(PLUGIN.parents[1]),
            }
        )
        result = subprocess.run(
            native_command([str(hook), "PreToolUse"]),
            input=payload,
            text=True,
            capture_output=True,
            env={**os.environ, "PLUGIN_ROOT": str(PLUGIN)},
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_hook_classifies_compiler_output_without_running_compiler(self) -> None:
        commands = (
            (
                "swiftc",
                "-parse-as-library",
                "-swift-version",
                "5",
                "-target",
                "arm64-apple-macosx27.0",
                "-module-cache-path",
                "/Users/REDACTED/.codex/visualizations/task-id/swift-cache",
                "-Xcc",
                "-fmodules-cache-path=/Users/REDACTED/.codex/visualizations/task-id/clang-cache",
                "-framework",
                "Speech",
                "-framework",
                "AVFoundation",
                "-framework",
                "CoreMedia",
                "-framework",
                "CryptoKit",
                "-o",
                "/Users/REDACTED/.codex/visualizations/task-id/probe",
                "/Users/REDACTED/.codex/visualizations/task-id/probe.swift",
            ),
            (
                "clang",
                "-fmodules-cache-path=/Users/REDACTED/.codex/visualizations/task-id/clang-cache",
                "-o",
                "/Users/REDACTED/.codex/visualizations/task-id/probe.o",
                "/Users/REDACTED/.codex/visualizations/task-id/probe.c",
            ),
        )
        for arguments in commands:
            command = shlex.join(arguments)
            with self.subTest(executable=arguments[0]):
                # This invokes only the policy hook; the compiler command stays inert text.
                self.assertEqual(self._classify(command), "", command)

    def test_hook_keeps_copied_and_symlinked_git_aliases_protected(self) -> None:
        source = shutil.which("git")
        if source is None:
            self.skipTest("Git is unavailable for executable-identity coverage")
        for alias_kind in ("copy", "symlink"):
            with (
                self.subTest(alias=alias_kind),
                tempfile.TemporaryDirectory() as directory,
            ):
                alias = Path(directory) / (
                    "git-copy.exe" if os.name == "nt" else "git-copy"
                )
                if alias_kind == "copy":
                    _ = shutil.copyfile(source, alias)
                    os.chmod(alias, Path(source).stat().st_mode & 0o777)
                else:
                    try:
                        _ = alias.symlink_to(source)
                    except OSError as error:
                        if os.name != "nt":
                            raise
                        self.skipTest(f"symbolic links unavailable: {error}")
                search_path = os.pathsep.join(
                    (directory, str(Path(source).parent), "/usr/bin")
                )
                command = f"PATH={shlex.quote(search_path)} git-copy reset --hard HEAD"
                self.assertIn('"permissionDecision":"deny"', self._classify(command))

    def test_hook_allows_a_direct_private_temp_log(self) -> None:
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
        for command in commands:
            with self.subTest(command=command):
                self.assertEqual(self._classify(command), "", command)

    def test_private_temp_redirection_keeps_credential_and_delete_denials(self) -> None:
        denied = (
            (
                "gh auth token > /private/tmp/codexy-1248-token.log",
                "CODEXY_DESTRUCTIVE_COMMAND_CREDENTIAL_EXPOSURE",
            ),
            (
                "python3 - <<EOF\n$(gh auth token)\nEOF",
                "CODEXY_DESTRUCTIVE_COMMAND_CREDENTIAL_EXPOSURE",
            ),
            (
                "sh -c 'sh -s' <<'EOF'\ngh auth token\nEOF",
                "CODEXY_DESTRUCTIVE_COMMAND_CREDENTIAL_EXPOSURE",
            ),
            (
                "sh -c 'sh -s' <<'EOF'\nrm -rf /\nEOF > /private/tmp/codexy-1248-reset.log",
                "CODEXY_DESTRUCTIVE_COMMAND_DESTRUCTIVE_EFFECT",
            ),
            (
                "(( 8 << 'EOF' ))\nrm -rf /\nEOF",
                "CODEXY_DESTRUCTIVE_COMMAND_DESTRUCTIVE_EFFECT",
            ),
        )
        for command, expected_code in denied:
            with self.subTest(command=command):
                output = self._classify(command)
                self.assertIn('"permissionDecision":"deny"', output, command)
                self.assertIn(expected_code, output, command)

    def test_hook_resolves_a_known_path_prefix_for_a_validator(self) -> None:
        command = (
            'PATH="$HOME/.cargo/bin:plugins/codexy-github/hooks:$PATH" '
            "codexy-merge-message-check.sh --expected-pr 1243 --expected-issue 1235 "
            "/private/tmp/codexy-merge-message.txt"
        )
        self.assertEqual(self._classify(command), "", command)

    def test_hook_rejects_a_private_temp_symlink_target(self) -> None:
        directory = Path("/private/tmp")
        if not directory.is_dir():
            self.skipTest("/private/tmp is unavailable")
        target = directory / f"codexy-issue-1248-{uuid4().hex}.log"
        _ = target.symlink_to(PLUGIN.parents[1] / "README.md")
        try:
            command = f"gh run view 1 --repo eunsoogi/codexy --log > {target}"
            self.assertIn('"permissionDecision":"deny"', self._classify(command))
        finally:
            target.unlink(missing_ok=True)

    def test_hook_rejects_a_private_temp_hard_link_target(self) -> None:
        directory = Path("/private/tmp")
        if not directory.is_dir():
            self.skipTest("/private/tmp is unavailable")
        target = directory / f"codexy-issue-1248-{uuid4().hex}.log"
        try:
            os.link(PLUGIN.parents[1] / "README.md", target)
        except OSError as error:
            self.skipTest(f"hard links unavailable for this workspace: {error}")
        try:
            command = f"gh run view 1 --repo eunsoogi/codexy --log > {target}"
            self.assertIn('"permissionDecision":"deny"', self._classify(command))
        finally:
            target.unlink(missing_ok=True)

    def test_private_temp_output_requires_current_user_ownership(self) -> None:
        directory = Path("/private/tmp")
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
