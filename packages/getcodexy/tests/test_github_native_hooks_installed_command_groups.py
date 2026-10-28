from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from github_native_hook_support import GithubNativeHookSupport, ROOT


class InstalledCommandGroupTests(GithubNativeHookSupport, unittest.TestCase):
    @unittest.skipUnless(shutil.which("codex"), "Codex host is required")
    def test_isolated_candidate_copy_handles_command_groups_safely(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "fresh Codex home"
            home.mkdir()
            environment = {**os.environ, "CODEX_HOME": str(home)}
            _ = self._host(environment, "plugin", "marketplace", "add", str(ROOT))
            plugin = self._host(environment, "plugin", "add", "codexy-github@codexy")
            installed_path = plugin.get("installedPath")
            if not isinstance(installed_path, str):
                raise AssertionError("Codex did not return an installed plugin path")
            installed = Path(installed_path)
            policy_sources = (
                "shell_brace_expansion.py",
                "shell_evaluator.py",
                "shell_redirections.py",
                "shell_segments.py",
            )
            for name in policy_sources:
                source = ROOT / "plugins/codexy-github/hooks/codexy_policy" / name
                packaged = installed / "hooks/codexy_policy" / name
                # Every changed parser file must come from this isolated candidate install.
                self.assertEqual(packaged.read_bytes(), source.read_bytes(), name)
            hook = installed / "hooks/codexy-destructive-command.sh"
            commands = (
                (
                    "tail -20 /tmp/codexy-1439-green.log && rustc --version && cargo --version "
                    "&& git diff --stat && git diff -- packages/codexy-runtime/src/version/readme.rs "
                    "&& rg -n '^name:|pull_request|cargo test|sync-plugin' "
                    ".github/workflows/{rust,plugin-config-validation,language-lint}.yml",
                    False,
                ),
                ("git diff --stat && printf x {one,two}", False),
                ("rm -rf .pytest_cache{,.old}", True),
                ("git status --short && { git reset --hard HEAD; }", True),
                ('git diff --stat && { eval "$PAYLOAD"; }', True),
                ("git diff -- stat {one,two}", False),
                ("git diff -- file{one,two}", False),
                ("git diff -- '{one,two}'", False),
                (r"git diff -- file\{one,two\}", False),
                ("git diff -- file{{one,two},{three,four}}", False),
                ("find . -type d -name __pycache__ -prune -exec rm -rf {} +", False),
                ("git diff --stat && r{m..m} -rf /tmp/codexy-review-unused", True),
                ("git diff --stat && git r{e..e}set --hard HEAD", True),
                ("git diff --stat && gh auth to{k..k}en", True),
                ('git diff --stat && e{v..v}al "$PAYLOAD"', True),
                ("git diff --stat && rm -{r,f} /tmp/codexy-review-unused", True),
                ("git diff --stat && git reset --{hard,soft} HEAD", True),
                ("git diff --stat && gh auth status --show-{token,help}", True),
                (
                    "git diff --stat && rm --recursiv{e..e} --forc{e..e} /tmp/codexy-review-unused",
                    True,
                ),
                ("git diff --stat && gh auth status --show-toke{n..n}", True),
                (
                    "git diff --stat && find /tmp/codexy-review-unused -delet{e..e}",
                    True,
                ),
                (
                    "git diff --stat && bash {-c,'rm -rf /tmp/codexy-review-unused'}",
                    True,
                ),
                ("A=m; r{$A,x} /tmp/codexy-review-unused", True),
                ("git diff -- file{*,one}", True),
                ("git diff -- {~,one}", True),
            )
            for event in ("PermissionRequest", "PreToolUse"):
                for command, denied in commands:
                    payload = json.dumps(
                        {
                            "hook_event_name": event,
                            "tool_name": "Bash",
                            "tool_input": {"command": command},
                            "cwd": str(ROOT),
                        }
                    )
                    # The installed hook classifies the payload; it never executes these commands.
                    result = subprocess.run(
                        [str(hook), event],
                        input=payload,
                        text=True,
                        capture_output=True,
                        env={**environment, "PLUGIN_ROOT": str(installed)},
                        check=False,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    if denied:
                        self.assertIn('"deny"', result.stdout, command)
                    else:
                        self.assertEqual(result.stdout, "", command)


if __name__ == "__main__":
    unittest.main()
