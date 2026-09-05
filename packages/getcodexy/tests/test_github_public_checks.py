from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class GithubPublicChecksTests(unittest.TestCase):
    def test_public_command_accepts_generic_title_label_and_merge_fixtures(
        self,
    ) -> None:
        self.run_check(
            "--check-issue-title", "--issue-title", "Extract GitHub workflow"
        )
        self.run_check(
            "--check-pr-title", "--pr-title", "refactor(github): extract workflow"
        )
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary) / "pr-state.json"
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
            self.run_check("--check-pr-labels", "--pr-state-file", str(state))
        self.run_check(
            "--check-merge-message",
            "--expected-issue",
            "553",
            "--expected-pr",
            "554",
            "--merge-message",
            "refactor(github): extract workflow (#554)\n\nFixes #553\n",
        )

    def test_public_command_rejects_invalid_title(self) -> None:
        result = self.run_check(
            "--check-pr-title",
            "--pr-title",
            "not a conventional title",
            expect=1,
        )
        self.assertIn("PR title must use Conventional Commit style", result.stderr)

    def test_public_command_enforces_issue_pr_and_merge_title_boundaries(self) -> None:
        for title in (
            "feat(task): desc",
            "feat(task)!: desc",
            "test(ci): measure Rust 1.95 costs",
        ):
            self.run_check("--check-pr-title", "--pr-title", title)
        for title in (
            "feat: desc",
            "feat(): desc",
            "feat(task): desc (#900)",
            "feat(task): desc #900",
            "feat(task): desc (PR #926)",
            "feat(task): desc PR #926",
            "feat(task): desc issue #926",
        ):
            self.run_check("--check-pr-title", "--pr-title", title, expect=1)
        for title in (
            "Reduce CI build time",
            "CI fails when cache restore times out",
            "Fix cache restoration after a runner restart",
            "Support HTTP/2 requests on port 8080",
            "Explain cache failures: retain the original error",
        ):
            self.run_check("--check-issue-title", "--issue-title", title)
        for title in (
            "CI: reduce build time",
            "CI : reduce build time",
            "Fix (task) : reject invalid titles",
            "CI： reduce build time",
            "CI - reduce build time",
            "CI – reduce build time",
            "CI — reduce build time",
            "[CI] Reduce build time",
            "CI",
            "Fix",
        ):
            self.run_check("--check-issue-title", "--issue-title", title, expect=1)
        self.run_check(
            "--check-merge-message",
            "--expected-pr",
            "926",
            "--expected-issue",
            "121",
            "--merge-message",
            "feat(task): desc (#926)\n\nFixes #121\n",
        )
        self.run_check(
            "--check-merge-message",
            "--expected-pr",
            "926",
            "--expected-issue",
            "121",
            "--merge-message",
            "feat(task): desc (#900)\n\nFixes #121\n",
            expect=1,
        )

    @staticmethod
    def run_check(*args: str, expect: int = 0) -> subprocess.CompletedProcess[str]:
        command = [sys.executable, "-m", "codexy_runtime_tools.github_checks", *args]
        result = subprocess.run(command, text=True, capture_output=True, check=False)
        if result.returncode != expect:
            raise AssertionError(
                f"{command} returned {result.returncode}: {result.stderr}"
            )
        return result


if __name__ == "__main__":
    unittest.main()
