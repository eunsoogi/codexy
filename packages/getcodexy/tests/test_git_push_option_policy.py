"""Regressions for the GitHub hook's Git push option policy."""

from __future__ import annotations

import importlib
import json
import os
import subprocess
import sys
import unittest

from github_native_hook_support import PLUGIN, ROOT, native_command

sys.path.insert(0, str(PLUGIN / "hooks"))
try:
    normalize_git_options = importlib.import_module(
        "codexy_policy.git_options"
    ).normalize
    evaluate_git_policy = importlib.import_module("codexy_policy.shell_git").evaluate
    ExecutionContext = importlib.import_module(
        "codexy_policy.execution_context_types"
    ).ExecutionContext
finally:
    sys.path.pop(0)


def evaluate_source_hook(command: str) -> str:
    """Feed a command string to the checked-in hook without executing Git."""
    payload = json.dumps(
        {
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": command},
            "cwd": str(ROOT),
        }
    )
    hook = PLUGIN / "hooks/codexy-destructive-command.sh"
    result = subprocess.run(
        native_command([str(hook), "PreToolUse"]),
        input=payload,
        text=True,
        capture_output=True,
        env={**os.environ, "PLUGIN_ROOT": str(PLUGIN)},
        check=False,
    )
    if result.returncode:
        raise AssertionError(result.stderr)
    return result.stdout


class GitPushOptionPolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.context = ExecutionContext(
            cwd=str(ROOT),
            cwd_owned=True,
            repository_status=True,
            git_dir=None,
            gh_repo=None,
        )

    def evaluate_push(self, arguments: list[str]) -> bool:
        denied, remote, alias = evaluate_git_policy(["push", *arguments], self.context)
        self.assertIsNone(remote)
        self.assertIsNone(alias)
        return denied

    def test_explicit_non_force_push_accepts_no_follow_tags(self) -> None:
        # This is the exact denied command from the issue, passed only as hook input.
        command = (
            "git push --no-follow-tags origin "
            "HEAD:refs/heads/eunsoogi/task-ownership-authority"
        )
        self.assertFalse(
            self.evaluate_push(
                [
                    "--no-follow-tags",
                    "origin",
                    "HEAD:refs/heads/eunsoogi/task-ownership-authority",
                ]
            )
        )
        self.assertEqual(evaluate_source_hook(command), "")

    def test_follow_tags_options_remain_distinct(self) -> None:
        refspec = "HEAD:refs/heads/push-safety-check"
        for option in ("--follow-tags", "--no-follow-tags"):
            with self.subTest(option=option):
                self.assertEqual(
                    normalize_git_options("push", [option, "origin", refspec]),
                    [option, "origin", refspec],
                )
                self.assertFalse(self.evaluate_push([option, "origin", refspec]))

    def test_destructive_and_unknown_push_forms_still_deny(self) -> None:
        arguments = [
            ["--delete", "origin", "topic"],
            ["--prune", "origin"],
            ["--all", "origin"],
            ["--tags", "origin"],
            ["--force", "origin", "HEAD:refs/heads/topic"],
            ["--force-with-lease", "origin", "HEAD:refs/heads/topic"],
            [
                "--force-with-lease=refs/heads/topic",
                "origin",
                "HEAD:refs/heads/topic",
            ],
            ["--force-if-includes", "origin", "HEAD:refs/heads/topic"],
            ["--mirror", "origin"],
            ["--stdin", "origin", "HEAD:refs/heads/topic"],
            ["--delete=topic", "origin"],
            ["--prune=all", "origin"],
            ["--all=true", "origin"],
            ["--tags=true", "origin"],
            ["--force=true", "origin", "HEAD:refs/heads/topic"],
            ["--force-if-includes=true", "origin", "HEAD:refs/heads/topic"],
            ["--mirror=true", "origin"],
            ["-d", "origin", "topic"],
            ["-D", "origin", "topic"],
            ["-A", "origin", "HEAD:refs/heads/topic"],
            ["-f", "origin", "HEAD:refs/heads/topic"],
            ["+HEAD:refs/heads/topic", "origin"],
            [":refs/heads/topic", "origin"],
            ["--no-follow-tags=1", "origin", "HEAD:refs/heads/topic"],
            ["--unsupported-danger", "origin", "HEAD:refs/heads/topic"],
        ]
        for arguments_for_push in arguments:
            with self.subTest(arguments=arguments_for_push):
                self.assertTrue(self.evaluate_push(arguments_for_push))


if __name__ == "__main__":
    unittest.main()
