from __future__ import annotations

import json
import os
import subprocess
import unittest

from github_native_hooks_installation import GithubNativeHooksInstallationMixin
from github_native_hook_support import PLUGIN, GithubNativeHookSupport

WINDOWS_KEYWORDS = tuple(
    "GitHub|issue|pull request|pull-request|pullrequest|review|merge".split("|")
)
CONTEXT_JSON = (
    '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":'
    '"Codexy GitHub workflow is installed. Use $git-workflow; GitHub authorization '
    'remains with the host, connector, and GitHub."}}'
)


class GithubNativeHooksTests(
    GithubNativeHooksInstallationMixin, GithubNativeHookSupport, unittest.TestCase
):
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

    def test_plugin_registers_title_checks_and_independent_bash_safety(self) -> None:
        hooks = json.loads((PLUGIN / "hooks/hooks.json").read_text(encoding="utf-8"))[
            "hooks"
        ]
        self.assertEqual(
            set(hooks), {"UserPromptSubmit", "PermissionRequest", "PreToolUse"}
        )
        context = hooks["UserPromptSubmit"]
        self.assertEqual(len(context), 1)
        self.assertIn("codexy-github-workflow-context.sh", json.dumps(context))
        for event in ("PermissionRequest", "PreToolUse"):
            groups = hooks[event]
            self.assertEqual(len(groups), 6, event)
            for group in groups[:5]:
                self.assertIn("codexy-title-check", json.dumps(group))
                self.assertEqual(group["hooks"][0]["timeout"], 5)
            self.assertEqual(groups[5]["matcher"], "^Bash$")
            self.assertIn("codexy-destructive-command", json.dumps(groups[5]))

        serialized = json.dumps(hooks)
        self.assertNotIn("plugin-version-bump", serialized)
        self.assertNotIn("codexy-repository-", serialized)
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
                for path in (PLUGIN / "hooks").iterdir()
                if path.is_file() and path.name.startswith("codexy-")
            },
            expected_hook_files,
        )

    def test_destructive_hook_keeps_credentials_and_local_safety(self) -> None:
        hook = PLUGIN / "hooks/codexy-destructive-command.sh"
        environment = {**os.environ, "PLUGIN_ROOT": str(PLUGIN)}
        for event in ("PermissionRequest", "PreToolUse"):
            for command in (
                "gh issue create --title arbitrary",
                "gh workflow run unrelated.yml --ref topic",
                "gh api --method POST repos/eunsoogi/codexy/releases -f tag_name=v1",
                "gh pr merge 1 --squash",
            ):
                payload = json.dumps(
                    {
                        "hook_event_name": event,
                        "tool_name": "Bash",
                        "tool_input": {"command": command},
                        "cwd": str(PLUGIN),
                    }
                )
                self.assertEqual(
                    self._run_process([str(hook), event], payload, environment),
                    "",
                    command,
                )
            for command in (
                "gh auth token",
                "gh auth status --show-token",
                "GH_TOKEN=fixture gh issue list",
                "gh api -H 'Authorization: Bearer fixture' repos/eunsoogi/codexy",
                "rm -rf /",
                "git reset --hard HEAD",
            ):
                payload = json.dumps(
                    {
                        "hook_event_name": event,
                        "tool_name": "Bash",
                        "tool_input": {"command": command},
                        "cwd": str(PLUGIN),
                    }
                )
                output = self._run_process([str(hook), event], payload, environment)
                self.assertIn('"deny"', output, command)


if __name__ == "__main__":
    unittest.main()
