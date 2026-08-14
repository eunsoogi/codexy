"""Regression coverage for real language-lint failure fixtures."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import unittest
from pathlib import Path


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

    def test_workflow_exercises_real_failure_fixtures_for_every_route(self) -> None:
        workflow = (ROOT / ".github/workflows/language-lint.yml").read_text(
            encoding="utf-8"
        )

        for language in LANGUAGES:
            with self.subTest(language=language):
                self.assertIn(
                    f"scripts/verify-lint-fixtures.py --language {language}", workflow
                )

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
