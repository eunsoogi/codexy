from __future__ import annotations

import json
import os
import subprocess
import unittest

from github_native_hook_support import PLUGIN


class GithubTitleHooksTests(unittest.TestCase):
    def _run_process(
        self,
        command: list[str],
        payload: str,
        environment: dict[str, str],
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

    def test_title_checks_cover_direct_shell_and_nested_paths_only(self) -> None:
        hook = str(PLUGIN / "hooks/codexy-title-check.sh")
        environment = {**os.environ, "PLUGIN_ROOT": str(PLUGIN)}
        cases = (
            (
                "issue",
                "mcp__codex_apps__github_create_issue",
                {"title": "Valid issue", "body": "free form"},
                False,
            ),
            (
                "issue",
                "mcp__codex_apps__github_create_issue",
                {"title": "fix: invalid issue", "body": "free form"},
                True,
            ),
            ("pr", "github.update_pull_request", {"body": "free form"}, False),
            (
                "pr",
                "github.update_pull_request",
                {"title": "plain title", "body": "free form"},
                True,
            ),
            (
                "shell",
                "Bash",
                {"command": "gh pr create --title 'fix(hooks): free body' --body note"},
                False,
            ),
            (
                "shell",
                "Bash",
                {"command": "gh pr create --title 'plain title' --body note"},
                True,
            ),
            (
                "nested",
                "functions.exec",
                {
                    "code": "await tools.mcp__codex_apps__github_create_issue({title: 'Valid issue', body: 'free form'});"
                },
                False,
            ),
            (
                "nested",
                "functions.exec",
                {
                    "code": "await tools.mcp__codex_apps__github_create_issue({title: 'fix: invalid', body: 'free form'});"
                },
                True,
            ),
            ("shell", "Bash", {"command": "gh release create v1"}, False),
        )
        for kind, tool, tool_input, denied in cases:
            with self.subTest(kind=kind, denied=denied):
                payload = json.dumps(
                    {
                        "hook_event_name": "PreToolUse",
                        "tool_name": tool,
                        "tool_input": tool_input,
                    }
                )
                output = self._run_process(
                    [hook, "PreToolUse", kind], payload, environment
                )
                self.assertEqual(bool(output), denied, output)

    def test_title_parser_ignores_data_literals_and_checks_graphql(self) -> None:
        hook = str(PLUGIN / "hooks/codexy-title-check.sh")
        environment = {**os.environ, "PLUGIN_ROOT": str(PLUGIN)}
        cases = (
            ("nested", "// github_create_issue({title: 'plain'})", False),
            ("nested", "const re = /github_create_issue\\({title: 'plain'}/;", False),
            ("nested", "eval(\"github_create_issue({title: 'plain'})\")", True),
            (
                "shell",
                "gh api graphql -f query='mutation { createIssue(input: {title: \"plain\"}) { issue { id } } }'",
                True,
            ),
            (
                "shell",
                "gh api graphql -f query='mutation { createIssue(input: {title: \"Valid issue\"}) { issue { id } } }'",
                False,
            ),
            (
                "shell",
                "gh api --method POST graphql -f query='mutation { createIssue(input: {title: \"plain\"}) { issue { id } } }'",
                True,
            ),
            (
                "shell",
                "gh api --method POST graphql -f query='mutation { createIssue(input: {title: \"Valid issue\"}) { issue { id } } }'",
                False,
            ),
        )
        for kind, value, denied in cases:
            tool = "functions.exec" if kind == "nested" else "Bash"
            field = "code" if kind == "nested" else "command"
            payload = json.dumps(
                {
                    "hook_event_name": "PreToolUse",
                    "tool_name": tool,
                    "tool_input": {field: value},
                }
            )
            with self.subTest(kind=kind, value=value):
                output = self._run_process(
                    [hook, "PreToolUse", kind], payload, environment
                )
                self.assertEqual(bool(output), denied, output)


if __name__ == "__main__":
    unittest.main()
