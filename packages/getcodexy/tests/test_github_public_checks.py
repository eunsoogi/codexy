from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


class GithubPublicChecksTests(unittest.TestCase):
    def test_public_command_accepts_generic_title_label_and_merge_fixtures(self) -> None:
        self.run_check("--check-issue-title", "--issue-title", "Extract GitHub workflow")
        self.run_check("--check-pr-title", "--pr-title", "refactor(github): extract workflow")
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary) / "pr-state.json"
            state.write_text(json.dumps({
                "number": 553, "state": "OPEN", "repository": "owner/repo",
                "labels": [{"name": "type/refactor"}],
                "repositoryLabels": [{"name": "type/refactor"}],
            }), encoding="utf-8")
            self.run_check("--check-pr-labels", "--pr-state-file", str(state))
        self.run_check(
            "--check-merge-message", "--expected-issue", "553", "--expected-pr", "554",
            "--merge-message", "refactor(github): extract workflow (#554)\n\nFixes #553\n",
        )

    def test_public_command_rejects_invalid_title(self) -> None:
        result = self.run_check(
            "--check-pr-title", "--pr-title", "not a conventional title", expect=1,
        )
        self.assertIn("PR title must use Conventional Commit style", result.stderr)

    @staticmethod
    def run_check(*args: str, expect: int = 0) -> subprocess.CompletedProcess[str]:
        command = ["python", "-m", "codexy_runtime_tools.github_checks", *args]
        result = subprocess.run(command, text=True, capture_output=True, check=False)
        if result.returncode != expect:
            raise AssertionError(f"{command} returned {result.returncode}: {result.stderr}")
        return result


if __name__ == "__main__":
    unittest.main()
