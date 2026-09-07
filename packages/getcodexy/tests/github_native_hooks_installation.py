"""Installed-plugin lifecycle coverage for the retained GitHub component surface."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from github_native_hook_support import ROOT


class GithubNativeHooksInstallationMixin:
    @unittest.skipUnless(shutil.which("codex"), "Codex host is required")
    def test_isolated_direct_install_exposes_only_retained_github_artifacts(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "fresh Codex home"
            home.mkdir()
            workspace = home / "unrelated repository"
            (workspace / ".git").mkdir(parents=True)
            environment = {**os.environ, "CODEX_HOME": str(home)}
            self._host(environment, "plugin", "marketplace", "add", str(ROOT))
            core = self._host(environment, "plugin", "add", "codexy@codexy")
            self._assert_enabled_plugins(
                self._host(environment, "plugin", "list"),
                {"codexy@codexy"},
            )
            self.assertFalse(
                (Path(core["installedPath"]) / "skills/git-workflow").exists()
            )
            github = self._host(environment, "plugin", "add", "codexy-github@codexy")
            self._assert_enabled_plugins(
                self._host(environment, "plugin", "list"),
                {"codexy@codexy", "codexy-github@codexy"},
            )
            installed = Path(github["installedPath"])
            hook_root = installed / "hooks"
            self.assertTrue((installed / "skills/git-workflow/SKILL.md").is_file())
            self.assertTrue((installed / "agents/codexy-weaver.toml").is_file())
            hooks = json.loads((hook_root / "hooks.json").read_text(encoding="utf-8"))[
                "hooks"
            ]
            self.assertEqual(
                set(hooks), {"UserPromptSubmit", "PermissionRequest", "PreToolUse"}
            )
            for event in ("PermissionRequest", "PreToolUse"):
                self.assertEqual(len(hooks[event]), 6)
                self.assertTrue(
                    all(
                        "codexy-title-check" in json.dumps(group)
                        for group in hooks[event][:5]
                    )
                )
                self.assertEqual(hooks[event][5]["matcher"], "^Bash$")
            self.assertIn("codexy-destructive-command", json.dumps(hooks))
            self.assertIn(
                "$git-workflow",
                self._run_process(
                    [str(hook_root / "codexy-github-workflow-context.sh")],
                    json.dumps({"prompt": "Open a GitHub issue"}),
                    {**environment, "PLUGIN_ROOT": str(installed)},
                ),
            )

            for event in ("PermissionRequest", "PreToolUse"):
                for kind, tool, field, value, denied in (
                    (
                        "issue",
                        "mcp__codex_apps__github_create_issue",
                        "title",
                        "Valid issue",
                        False,
                    ),
                    (
                        "issue",
                        "mcp__codex_apps__github_create_issue",
                        "title",
                        "fix: invalid",
                        True,
                    ),
                    ("pr", "github.update_pull_request", "body", "free form", False),
                    ("pr", "github.update_pull_request", "title", "plain title", True),
                    (
                        "shell",
                        "Bash",
                        "command",
                        "gh pr create --title 'fix(hooks): installed'",
                        False,
                    ),
                    (
                        "shell",
                        "Bash",
                        "command",
                        "gh pr create --title 'plain title'",
                        True,
                    ),
                    (
                        "nested",
                        "functions.exec",
                        "code",
                        "tools.mcp__codex_apps__github_create_issue({title: 'Valid issue'})",
                        False,
                    ),
                    (
                        "nested",
                        "functions.exec",
                        "code",
                        "tools.mcp__codex_apps__github_create_issue({title: 'fix: invalid'})",
                        True,
                    ),
                ):
                    output = self._run_process(
                        [str(hook_root / "codexy-title-check.sh"), event, kind],
                        json.dumps(
                            {
                                "hook_event_name": event,
                                "tool_name": tool,
                                "tool_input": {field: value},
                            }
                        ),
                        {**environment, "PLUGIN_ROOT": str(installed)},
                    )
                    self.assertEqual(bool(output), denied, f"{event} {kind}: {value}")
                for command in (
                    "gh issue create --title arbitrary",
                    "gh workflow run unrelated.yml --ref topic",
                    "gh api --method POST repos/example/project/releases -f tag_name=v1",
                ):
                    output = self._run_process(
                        [str(hook_root / "codexy-destructive-command.sh"), event],
                        json.dumps(
                            {
                                "hook_event_name": event,
                                "tool_name": "Bash",
                                "tool_input": {"command": command},
                                "cwd": str(workspace),
                            }
                        ),
                        {**environment, "PLUGIN_ROOT": str(installed)},
                    )
                    self.assertEqual(output, "", command)
                for command in (
                    "gh auth token",
                    "GH_TOKEN=fixture gh issue list",
                    "rm -rf /",
                ):
                    output = self._run_process(
                        [str(hook_root / "codexy-destructive-command.sh"), event],
                        json.dumps(
                            {
                                "hook_event_name": event,
                                "tool_name": "Bash",
                                "tool_input": {"command": command},
                                "cwd": str(workspace),
                            }
                        ),
                        {**environment, "PLUGIN_ROOT": str(installed)},
                    )
                    self.assertIn('"deny"', output, command)

            expected_hook_files = {
                "codexy-destructive-command.cmd",
                "codexy-destructive-command.py",
                "codexy-destructive-command.sh",
                "codexy-github-workflow-context.cmd",
                "codexy-github-workflow-context.ps1",
                "codexy-github-workflow-context.sh",
                "codexy-hook-runtime.sh",
                "codexy-title-check.cmd",
                "codexy-title-check.py",
                "codexy-title-check.sh",
                "codexy-issue-title-check.sh",
                "codexy-merge-message-check.sh",
                "codexy-pr-label-check.sh",
                "codexy-pr-title-check.sh",
                "codexy-readiness-guard-json.sh",
                "codexy-readiness-guard-pr-labels.sh",
                "codexy-readiness-guard-values.sh",
                "codexy-readiness-guard.sh",
                "codexy-title-policy.sh",
            }
            self.assertEqual(
                {
                    path.name
                    for path in hook_root.iterdir()
                    if path.is_file() and path.name.startswith("codexy-")
                },
                expected_hook_files,
            )

            self._run(
                installed / "hooks/codexy-issue-title-check.sh",
                "--issue-title",
                "Extract GitHub workflow",
            )
            self._run(
                installed / "hooks/codexy-pr-title-check.sh",
                "--pr-title",
                "refactor(github): extract workflow",
            )
            state = home / "captured PR state.json"
            state.write_text(
                json.dumps(
                    {
                        "number": 553,
                        "state": "OPEN",
                        "repository": "owner/repo",
                        "labels": [{"name": "type/refactor"}],
                        "repositoryLabels": [{"name": "type/refactor"}],
                    }
                ),
                encoding="utf-8",
            )
            self._run(
                installed / "hooks/codexy-pr-label-check.sh",
                "--pr-state-file",
                str(state),
            )
            self._run(
                installed / "hooks/codexy-merge-message-check.sh",
                "--expected-issue",
                "553",
                "--expected-pr",
                "554",
                "--merge-message",
                "refactor(github): extract workflow (#554)\n\nFixes #553\n",
            )

    @staticmethod
    def _run(path: Path, *arguments: str) -> None:
        result = subprocess.run(
            [str(path), *arguments], text=True, capture_output=True, check=False
        )
        if result.returncode:
            raise AssertionError(f"{path.name} failed:\n{result.stdout}{result.stderr}")
