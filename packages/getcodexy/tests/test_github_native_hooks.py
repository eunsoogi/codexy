from __future__ import annotations

import json
import os
import shutil
import subprocess
import unittest

from github_native_hooks_installation import GithubNativeHooksInstallationMixin
from github_native_hook_support import PLUGIN, ROOT, GithubNativeHookSupport

WINDOWS_KEYWORDS = tuple(
    "GitHub|issue|pull request|pull-request|pullrequest|review|merge".split("|")
)
CONTEXT_JSON = (
    '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":'
    '"Codexy GitHub workflow is installed. Use $git-workflow; its package-owned '
    'generic admission hooks are active."}}'
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


if __name__ == "__main__":
    unittest.main()
