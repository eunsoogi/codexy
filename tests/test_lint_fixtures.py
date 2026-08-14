"""Regression coverage for real language-lint failure fixtures."""

from __future__ import annotations

import importlib.util
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/lint-repository.py"
LANGUAGES = {
    "rust",
    "python",
    "shell",
    "powershell",
    "windows-command",
    "text",
}


def inventory_module():
    spec = importlib.util.spec_from_file_location("lint_repository", RUNNER)
    if spec is None or spec.loader is None:
        raise AssertionError("lint repository runner is not importable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class LintFixtureTests(unittest.TestCase):
    def test_malformed_fixtures_are_excluded_from_every_route(self) -> None:
        runner = inventory_module()

        for language in LANGUAGES:
            with self.subTest(language=language):
                self.assertFalse(
                    any(
                        path.startswith("tests/lint-fixtures/")
                        for path in runner.inventory_files(ROOT, language)
                    )
                )

    def test_text_plan_excludes_malformed_fixture_paths(self) -> None:
        runner = inventory_module()

        for mode in ("check", "fix"):
            with self.subTest(mode=mode):
                plan = runner.build_plan(ROOT, mode, {"text"})
                self.assertTrue(
                    all(
                        not path.startswith("tests/lint-fixtures/")
                        for step in plan
                        for path in step.command
                    )
                )

    def test_changed_scope_skips_unchanged_repository_debt(self) -> None:
        runner = inventory_module()

        with mock.patch.dict(os.environ, {"CODEXY_LINT_CHANGED_SINCE": "HEAD"}):
            self.assertEqual(runner.build_plan(ROOT, "check", LANGUAGES), [])

    def test_changed_scope_includes_a_changed_maintained_file(self) -> None:
        runner = inventory_module()

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "unchanged.py").write_text("value = 1\n", encoding="utf-8")
            subprocess.run(
                ["git", "init", "--quiet", "--initial-branch=main"],
                cwd=root,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.email", "lint@example.invalid"],
                cwd=root,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Lint test"], cwd=root, check=True
            )
            subprocess.run(["git", "add", "unchanged.py"], cwd=root, check=True)
            subprocess.run(
                ["git", "commit", "--quiet", "-m", "initial"], cwd=root, check=True
            )
            baseline = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=root,
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
            (root / "changed.py").write_text("value = 2\n", encoding="utf-8")
            subprocess.run(["git", "add", "changed.py"], cwd=root, check=True)
            subprocess.run(
                ["git", "commit", "--quiet", "-m", "changed"], cwd=root, check=True
            )

            with mock.patch.dict(os.environ, {"CODEXY_LINT_CHANGED_SINCE": baseline}):
                self.assertEqual(
                    runner.tracked_regular_files(root, "*.py"), ("changed.py",)
                )

    def test_changed_scope_reports_an_unavailable_baseline(self) -> None:
        runner = inventory_module()

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "script.py").write_text("value = 1\n", encoding="utf-8")
            subprocess.run(["git", "init", "--quiet"], cwd=root, check=True)
            subprocess.run(["git", "add", "script.py"], cwd=root, check=True)
            with mock.patch.dict(
                os.environ, {"CODEXY_LINT_CHANGED_SINCE": "missing-lint-baseline"}
            ):
                with self.assertRaisesRegex(ValueError, "baseline is unavailable"):
                    runner.tracked_regular_files(root, "*.py")

    def test_changed_scope_excludes_current_base_branch_edits(self) -> None:
        runner = inventory_module()

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "skill.md").write_text("original\n", encoding="utf-8")
            subprocess.run(
                ["git", "init", "--quiet", "--initial-branch=main"],
                cwd=root,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.email", "lint@example.invalid"],
                cwd=root,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Lint test"], cwd=root, check=True
            )
            subprocess.run(["git", "add", "skill.md"], cwd=root, check=True)
            subprocess.run(
                ["git", "commit", "--quiet", "-m", "initial"], cwd=root, check=True
            )
            subprocess.run(["git", "branch", "feature"], cwd=root, check=True)
            (root / "skill.md").write_text("base update\n", encoding="utf-8")
            subprocess.run(
                ["git", "commit", "-am", "base update"], cwd=root, check=True
            )
            subprocess.run(
                ["git", "switch", "--quiet", "feature"], cwd=root, check=True
            )
            (root / "feature.py").write_text("value = 1\n", encoding="utf-8")
            subprocess.run(["git", "add", "feature.py"], cwd=root, check=True)
            subprocess.run(
                ["git", "commit", "--quiet", "-m", "feature"], cwd=root, check=True
            )

            with mock.patch.dict(os.environ, {"CODEXY_LINT_CHANGED_SINCE": "main"}):
                self.assertEqual(
                    runner.tracked_regular_files(root, "*.md", "*.py"),
                    ("feature.py",),
                )

    def test_workflow_exercises_real_failure_fixtures_for_every_route(self) -> None:
        workflow = (ROOT / ".github/workflows/language-lint.yml").read_text(
            encoding="utf-8"
        )

        for language in LANGUAGES:
            with self.subTest(language=language):
                self.assertIn(
                    f"scripts/verify-lint-fixtures.py --language {language}", workflow
                )

        for job in ("rust", "python", "text", "shell", "windows"):
            with self.subTest(job=job):
                section = re.search(rf"(?ms)^  {job}:.*?(?=^  [a-z]+:\n|\Z)", workflow)
                self.assertIsNotNone(section)
                self.assertIn("uses: actions/checkout@v7", section.group())
                self.assertIn("fetch-depth: 0", section.group())

        self.assertIn(
            "ref: ${{ github.event.pull_request.head.sha || github.sha }}", workflow
        )
        self.assertIn(
            "github.event.pull_request && format('origin/{0}', github.event.pull_request.base.ref)",
            workflow,
        )

    def test_shell_launcher_avoids_ambiguous_empty_environment_assignment(self) -> None:
        launcher = (ROOT / "scripts/lint-repository").read_text(encoding="utf-8")

        self.assertNotIn("CDPATH= cd", launcher)
        self.assertIn('cd -- "$(dirname -- "$0")" || exit 1', launcher)

    def test_powershell_fixture_exercises_multiple_paths(self) -> None:
        fixtures = (ROOT / "scripts/verify-lint-fixtures.py").read_text(
            encoding="utf-8"
        )

        self.assertIn('"tests/lint-fixtures/powershell/good.ps1",', fixtures)
        self.assertIn('"tests/lint-fixtures/powershell/bad.ps1",', fixtures)
        self.assertIn("A positional parameter cannot be found", fixtures)
        self.assertNotIn('"-Path",', fixtures)

    def test_windows_command_fixtures_cover_success_and_failure(self) -> None:
        checker = [sys.executable, "scripts/lint-windows-command.py"]
        valid = subprocess.run(
            [*checker, "tests/lint-fixtures/windows-command/valid.cmd"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        invalid = subprocess.run(
            [*checker, "tests/lint-fixtures/windows-command/bad.cmd"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )

        self.assertEqual(valid.returncode, 0, valid.stderr)
        self.assertNotEqual(invalid.returncode, 0)
        self.assertIn("unsupported launcher syntax", invalid.stderr)


if __name__ == "__main__":
    unittest.main()
