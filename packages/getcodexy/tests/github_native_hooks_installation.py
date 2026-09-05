"""Installed-plugin lifecycle coverage shared by the native hook test entrypoint."""

from __future__ import annotations

import json
import os
import shutil
import tempfile
import unittest
from pathlib import Path

from github_nested_exec_support import assert_nested_exec_cases
from github_native_hook_support import ROOT


class GithubNativeHooksInstallationMixin:
    @unittest.skipUnless(shutil.which("codex"), "Codex host is required")
    def test_isolated_direct_install_exposes_only_installed_github_artifacts(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "fresh Codex home"
            home.mkdir()
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
            self.assertFalse(
                (Path(core["installedPath"]) / "agents/codexy-weaver.toml").exists()
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
            self.assertTrue((hook_root / "hooks.json").is_file())
            self.assertIn(
                "$git-workflow",
                self._run_process(
                    [str(hook_root / "codexy-github-workflow-context.sh")],
                    json.dumps({"prompt": "Open a GitHub issue"}),
                    {**environment, "PLUGIN_ROOT": str(installed)},
                ),
            )
            admissions = (
                ("issue", "feat(github): extract workflow", True),
                ("issue", "Extract GitHub workflow", False),
                ("issue", "CI : reduce build time", True),
                ("issue", "CI", True),
                ("pr", "Extract GitHub workflow", True),
                ("pr", "refactor(github): extract workflow", False),
                ("pr", "feat: desc", True),
                ("pr", "feat(task): desc (#900)", True),
                ("pr", "feat(task): desc PR #900", True),
            )
            for rule, title, denied in admissions:
                self._admission(installed, environment, rule, title, denied)
            self._admission_payload(
                installed,
                environment,
                "issue",
                {"cwd": "A", "tool_input": {"title": "extract workflow"}},
                True,
            )
            self._admission_payload(
                installed,
                environment,
                "issue",
                {
                    "session_id": "session",
                    "transcript_path": "/tmp/transcript",
                    "model": "gpt-5.6-terra",
                    "turn_id": "turn",
                    "permission_mode": "default",
                    "tool_use_id": "tool",
                    "tool_name": "mcp__codex_apps__github_create_issue",
                    "tool_input": {"title": "Extract workflow"},
                },
                False,
            )
            self._admission_payload(
                installed,
                environment,
                "pr",
                {"cwd": "fix: decoy", "tool_input": {"title": "Extract workflow"}},
                True,
            )
            raw_admissions = (
                (
                    '{"tool_input":{"title":"Extract workflow","title":"extract workflow"}}',
                    True,
                ),
                ('{"tool_input":{"title":"Extract\\u0020workflow"}}', False),
                ("{", True),
            )
            for payload, denied in raw_admissions:
                self._admission_raw(installed, environment, "issue", payload, denied)
            self._admission_payload(
                installed,
                environment,
                "issue",
                {
                    "tool_input": {
                        "title": "Extract workflow",
                        "body": "x" * (64 * 1024),
                    }
                },
                True,
            )
            unavailable = self._run_process(
                [str(hook_root / "codexy-github-admission.sh"), "--rule", "issue"],
                "{}",
                environment,
            )
            self.assertIn("permissionDecision", unavailable)
            repository_payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "mcp__codex_apps__github_create_issue",
                "tool_input": {
                    "repository_full_name": "eunsoogi/codexy",
                    "title": "Require typed connector ownership",
                    "body": "## Problem\n\n## Scope\n\n## Acceptance Criteria\n\n## Verification",
                },
                "cwd": str(ROOT),
            }
            repository_hook = hook_root / "codexy-repository-issue.sh"
            self.assertEqual(
                self._run_process(
                    [str(repository_hook), "PreToolUse"],
                    json.dumps(repository_payload),
                    {**environment, "PLUGIN_ROOT": str(installed)},
                ),
                "",
            )
            assert_nested_exec_cases(
                self, self._run_process, installed, environment, ROOT
            )
            title_checks = {
                "issue": "Extract GitHub workflow",
                "pr": "refactor(github): extract workflow",
            }
            for kind, title in title_checks.items():
                title_check = hook_root / f"codexy-{kind}-title-check.sh"
                self._run(title_check, f"--{kind}-title", title)
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
