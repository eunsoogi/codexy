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
            self._host(environment, "plugin", "marketplace", "add", str(ROOT))
            plugin = self._host(environment, "plugin", "add", "codexy-github@codexy")
            installed_path = plugin.get("installedPath")
            self.assertIsInstance(installed_path, str)
            installed = Path(installed_path)
            source = (
                ROOT / "plugins/codexy-github/hooks/codexy_policy/shell_segments.py"
            )
            packaged = installed / "hooks/codexy_policy/shell_segments.py"
            # Confirm the isolated host copied this branch's parser, not another cached version.
            self.assertEqual(packaged.read_bytes(), source.read_bytes())
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
