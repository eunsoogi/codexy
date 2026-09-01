from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from github_native_hook_support import PLUGIN, ROOT, GithubNativeHookSupport

WINDOWS_KEYWORDS = tuple(
    "GitHub|issue|pull request|pull-request|pullrequest|review|merge".split("|")
)
CONTEXT_JSON = (
    '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":'
    '"Codexy GitHub workflow is installed. Use $git-workflow; its package-owned '
    'generic admission hooks are active."}}'
)


class GithubNativeHooksTests(GithubNativeHookSupport, unittest.TestCase):
    def _run_process(
        self,
        command: list[str],
        payload: str,
        environment: dict[str, str] | None = None,
    ) -> str:
        result = subprocess.run(
            command,
            input=payload,
            text=True,
            capture_output=True,
            env=environment,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_windows_workflow_context_is_native_and_exact(self) -> None:
        launcher = (PLUGIN / "hooks/codexy-github-workflow-context.cmd").read_text(
            encoding="utf-8"
        )
        self.assertNotIn("powershell", launcher.lower())
        self.assertNotIn(">", launcher)
        self.assertIn("%SystemRoot%\\System32\\findstr.exe", launcher)
        for keyword in WINDOWS_KEYWORDS:
            self.assertIn(f'/c:"{keyword.lower()}"', launcher.lower())
        self.assertIn(f"echo {CONTEXT_JSON}", launcher)

    def test_workflow_context_preserves_prompt_parity(self) -> None:
        if os.name == "nt":
            matching_prompts = WINDOWS_KEYWORDS
            hook = str(PLUGIN / "hooks/codexy-github-workflow-context.cmd")
            command, environment = ["cmd.exe", "/d", "/c", hook], None
        else:
            matching_prompts = ("Create a GitHub pull request",)
            hook = str(PLUGIN / "hooks/codexy-github-workflow-context.sh")
            command = [hook]
            environment = {**os.environ, "PLUGIN_ROOT": str(PLUGIN)}
        for prompt in (*matching_prompts, "Explain a Python list"):
            with self.subTest(prompt=prompt):
                payload = json.dumps({"prompt": prompt})
                expected = (
                    "" if prompt == "Explain a Python list" else f"{CONTEXT_JSON}\n"
                )
                self.assertEqual(
                    self._run_process(command, payload, environment), expected
                )

    def test_plugin_declares_host_discovered_workflow_hook(self) -> None:
        hooks = json.loads((PLUGIN / "hooks/hooks.json").read_text(encoding="utf-8"))
        command = hooks["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"]
        self.assertIn("${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.sh", command)
        self.assertIn(
            "commandWindows", hooks["hooks"]["UserPromptSubmit"][0]["hooks"][0]
        )
        self.assertEqual(
            self._admission_contract(hooks["hooks"]["PreToolUse"]),
            self.expected_pre_tool_use_admissions(),
        )
        windows = (PLUGIN / "hooks/codexy-github-admission-issue.cmd").read_text(
            encoding="utf-8"
        )
        self.assertIn("DisableDelayedExpansion", windows)
        self.assertIn("%SystemRoot%\\System32\\WindowsPowerShell", windows)
        self.assertNotIn("%*", windows)

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
                ("pr", "Extract GitHub workflow", True),
                ("pr", "refactor(github): extract workflow", False),
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


if __name__ == "__main__":
    unittest.main()
