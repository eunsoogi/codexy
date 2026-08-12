from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins" / "codexy-github"


class GithubNativeHooksTests(unittest.TestCase):
    def test_plugin_declares_host_discovered_workflow_hook(self) -> None:
        hooks = json.loads((PLUGIN / "hooks/hooks.json").read_text(encoding="utf-8"))
        command = hooks["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"]
        self.assertIn("${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.sh", command)
        self.assertIn("commandWindows", hooks["hooks"]["UserPromptSubmit"][0]["hooks"][0])
        admissions = hooks["hooks"]["PreToolUse"]
        self.assertEqual(len(admissions), 2)
        for admission in admissions:
            command = admission["hooks"][0]["command"]
            self.assertIn("${PLUGIN_ROOT}/hooks/codexy-github-admission.sh", command)
            self.assertIn("commandWindows", admission["hooks"][0])
        windows = (PLUGIN / "hooks/codexy-github-admission-issue.cmd").read_text(encoding="utf-8")
        self.assertIn("DisableDelayedExpansion", windows)
        self.assertIn("%SystemRoot%\\System32\\WindowsPowerShell", windows)
        self.assertNotIn("%*", windows)
        self.assertIn("commandWindows", hooks["hooks"]["PreToolUse"][0]["hooks"][0])

    def test_host_resolved_plugin_hook_adds_context_only_for_github_prompts(self) -> None:
        command = [str(PLUGIN / "hooks/codexy-github-workflow-context.sh")]
        environment = {**os.environ, "PLUGIN_ROOT": str(PLUGIN)}
        github = subprocess.run(
            command, input=json.dumps({"prompt": "Create a GitHub pull request"}),
            text=True, capture_output=True, env=environment, check=False,
        )
        self.assertEqual(github.returncode, 0, github.stderr)
        self.assertIn("$git-workflow", github.stdout)
        unrelated = subprocess.run(
            command, input=json.dumps({"prompt": "Explain a Python list"}),
            text=True, capture_output=True, env=environment, check=False,
        )
        self.assertEqual(unrelated.returncode, 0, unrelated.stderr)
        self.assertEqual(unrelated.stdout, "")

    @unittest.skipUnless(shutil.which("codex"), "Codex host is required")
    def test_isolated_direct_install_exposes_only_installed_github_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "fresh Codex home"
            home.mkdir()
            environment = {**os.environ, "CODEX_HOME": str(home)}
            self._host(environment, "plugin", "marketplace", "add", str(ROOT))
            core = self._host(environment, "plugin", "add", "codexy@codexy")
            self._assert_enabled_plugins(
                self._host(environment, "plugin", "list"), {"codexy@codexy"},
            )
            self.assertFalse((Path(core["installedPath"]) / "skills/git-workflow").exists())
            self.assertFalse((Path(core["installedPath"]) / "agents/codexy-weaver.toml").exists())
            github = self._host(environment, "plugin", "add", "codexy-github@codexy")
            self._assert_enabled_plugins(
                self._host(environment, "plugin", "list"),
                {"codexy@codexy", "codexy-github@codexy"},
            )
            installed = Path(github["installedPath"])
            self.assertTrue((installed / "skills/git-workflow/SKILL.md").is_file())
            self.assertTrue((installed / "agents/codexy-weaver.toml").is_file())
            self.assertTrue((installed / "hooks/hooks.json").is_file())
            result = subprocess.run(
                [str(installed / "hooks/codexy-github-workflow-context.sh")],
                input=json.dumps({"prompt": "Open a GitHub issue"}), text=True,
                capture_output=True, env={**environment, "PLUGIN_ROOT": str(installed)}, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("$git-workflow", result.stdout)
            self._admission(installed, environment, "issue", "feat(github): extract workflow", True)
            self._admission(installed, environment, "issue", "Extract GitHub workflow", False)
            self._admission(installed, environment, "pr", "Extract GitHub workflow", True)
            self._admission(installed, environment, "pr", "refactor(github): extract workflow", False)
            self._admission_payload(installed, environment, "issue", {"cwd": "A", "tool_input": {"title": "extract workflow"}}, True)
            self._admission_payload(installed, environment, "issue", {
                "session_id": "session", "transcript_path": "/tmp/transcript", "model": "gpt-5.6-terra",
                "turn_id": "turn", "permission_mode": "default", "tool_use_id": "tool",
                "tool_name": "mcp__codex_apps__github_create_issue", "tool_input": {"title": "Extract workflow"},
            }, False)
            self._admission_payload(installed, environment, "pr", {"cwd": "fix: decoy", "tool_input": {"title": "Extract workflow"}}, True)
            self._admission_raw(installed, environment, "issue", '{"tool_input":{"title":"Extract workflow","title":"extract workflow"}}', True)
            self._admission_raw(installed, environment, "issue", '{"tool_input":{"title":"Extract\\u0020workflow"}}', False)
            self._admission_raw(installed, environment, "issue", '{', True)
            self._admission_payload(installed, environment, "issue", {"tool_input": {"title": "Extract workflow", "body": "x" * (64 * 1024)}}, True)
            unavailable = subprocess.run(
                [str(installed / "hooks/codexy-github-admission.sh"), "--rule", "issue"], input="{}",
                text=True, capture_output=True, env=environment, check=False,
            )
            self.assertEqual(unavailable.returncode, 0, unavailable.stderr)
            self.assertIn("permissionDecision", unavailable.stdout)
            repository_hook = subprocess.run(
                [str(installed / "hooks/codexy-repository-issue.sh"), "PreToolUse"],
                input=json.dumps({"hook_event_name": "PreToolUse", "tool_name": "mcp__codex_apps__github_create_issue", "tool_input": {"repository_full_name": "eunsoogi/codexy", "title": "Require typed connector ownership", "body": "## Problem\n\n## Scope\n\n## Acceptance Criteria\n\n## Verification"}, "cwd": str(ROOT)}),
                text=True, capture_output=True,
                env={**environment, "PLUGIN_ROOT": str(installed)}, check=False,
            )
            self.assertEqual(repository_hook.returncode, 0, repository_hook.stderr)
            self.assertEqual(repository_hook.stdout, "")
            self._run(installed / "hooks/codexy-issue-title-check.sh", "--issue-title", "Extract GitHub workflow")
            self._run(installed / "hooks/codexy-pr-title-check.sh", "--pr-title", "refactor(github): extract workflow")
            state = home / "captured PR state.json"
            state.write_text(json.dumps({"number": 553, "state": "OPEN", "repository": "owner/repo", "labels": [{"name": "type/refactor"}], "repositoryLabels": [{"name": "type/refactor"}]}), encoding="utf-8")
            self._run(installed / "hooks/codexy-pr-label-check.sh", "--pr-state-file", str(state))
            self._run(installed / "hooks/codexy-merge-message-check.sh", "--expected-issue", "553", "--expected-pr", "554", "--merge-message", "refactor(github): extract workflow (#554)\n\nFixes #553\n")

    @staticmethod
    def _admission(installed: Path, environment: dict[str, str], rule: str, title: str, denied: bool) -> None:
        GithubNativeHooksTests._admission_payload(
            installed, environment, rule,
            {"tool_name": "mcp__codex_apps__github_create_issue", "tool_input": {"title": title}}, denied,
        )

    @staticmethod
    def _admission_payload(installed: Path, environment: dict[str, str], rule: str, payload: dict[str, object], denied: bool) -> None:
        GithubNativeHooksTests._admission_raw(installed, environment, rule, json.dumps(payload), denied)

    @staticmethod
    def _admission_raw(installed: Path, environment: dict[str, str], rule: str, payload: str, denied: bool) -> None:
        result = subprocess.run(
            [str(installed / "hooks/codexy-github-admission.sh"), "--rule", rule],
            input=payload,
            text=True, capture_output=True,
            env={**environment, "PLUGIN_ROOT": str(installed)}, check=False,
        )
        if result.returncode:
            raise AssertionError(result.stderr)
        assert ("permissionDecision" in result.stdout) == denied, result.stdout

    @staticmethod
    def _run(path: Path, *arguments: str) -> None:
        result = subprocess.run([str(path), *arguments], text=True, capture_output=True, check=False)
        if result.returncode:
            raise AssertionError(f"{path.name} failed:\n{result.stdout}{result.stderr}")

    @staticmethod
    def _host(environment: dict[str, str], *arguments: str) -> dict[str, object]:
        result = subprocess.run(
            ["codex", *arguments, "--json"], env=environment, text=True,
            capture_output=True, check=False,
        )
        if result.returncode:
            raise AssertionError(f"codex {' '.join(arguments)} failed:\n{result.stdout}{result.stderr}")
        return json.loads(result.stdout)

    @staticmethod
    def _assert_enabled_plugins(inventory: dict[str, object], expected: set[str]) -> None:
        installed = inventory.get("installed")
        if not isinstance(installed, list):
            raise AssertionError(f"missing installed plugin inventory: {inventory}")
        enabled = {
            entry.get("pluginId")
            for entry in installed
            if isinstance(entry, dict) and entry.get("enabled") is True
        }
        assert enabled == expected, enabled


if __name__ == "__main__":
    unittest.main()
