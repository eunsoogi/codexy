"""Regression tests for the repository language-lint entry point."""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest import mock
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "lint-repository.py"
EXPECTED_LANGUAGES = {
    "rust",
    "python",
    "shell",
    "powershell",
    "windows-command",
    "text",
}


def load_runner():
    spec = importlib.util.spec_from_file_location("lint_repository", SCRIPT)
    if spec is None or spec.loader is None:
        raise AssertionError("lint repository runner is not importable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class LintRepositoryTests(unittest.TestCase):
    def test_inventory_covers_every_maintained_language(self) -> None:
        runner = load_runner()

        self.assertEqual(set(runner.LANGUAGES), EXPECTED_LANGUAGES)
        runner.validate_inventory(runner.LANGUAGES)

    def test_inventory_rejects_an_omitted_language(self) -> None:
        runner = load_runner()
        incomplete = dict(runner.LANGUAGES)
        incomplete.pop("windows-command")

        with self.assertRaises(ValueError):
            runner.validate_inventory(incomplete)

    def test_check_and_fix_plans_remain_distinct(self) -> None:
        runner = load_runner()

        check = runner.build_plan(ROOT, "check", EXPECTED_LANGUAGES)
        fix = runner.build_plan(ROOT, "fix", EXPECTED_LANGUAGES)

        self.assertEqual({step.language for step in check}, EXPECTED_LANGUAGES)
        self.assertEqual({step.language for step in fix}, EXPECTED_LANGUAGES)
        self.assertTrue(all(step.read_only for step in check))
        self.assertTrue(any(not step.read_only for step in fix))

    def test_python_format_fix_does_not_repeat_the_subcommand(self) -> None:
        runner = load_runner()

        plan = runner.build_plan(ROOT, "fix", {"python"})
        formatter = next(
            step for step in plan if step.command[:2] == ("ruff", "format")
        )

        self.assertNotIn("format", formatter.command[2:])
        checker = next(step for step in plan if step.command[:2] == ("ruff", "check"))
        self.assertIn("--fix", checker.command)

    def test_rust_plan_scopes_clippy_to_changed_rust_sources(self) -> None:
        runner = load_runner()
        source = "packages/codexy-runtime/tests/repository_eol_contract.rs"

        with mock.patch("lint_repository_plan.selected_files", return_value=(source,)):
            plan = runner.build_plan(ROOT, "check", {"rust"})

        self.assertEqual(plan[0].command[:2], ("rustfmt", "+1.85.0"))
        self.assertIn("skip_children=true", plan[0].command)
        self.assertEqual(plan[1].command[:2], (sys.executable, "scripts/lint-rust.py"))
        self.assertIn(source, plan[0].command)
        self.assertIn(source, plan[1].command)

    def test_check_fix_fix_check_plan_is_idempotent(self) -> None:
        runner = load_runner()

        check_before = runner.build_plan(ROOT, "check", EXPECTED_LANGUAGES)
        fix_once = runner.build_plan(ROOT, "fix", EXPECTED_LANGUAGES)
        fix_twice = runner.build_plan(ROOT, "fix", EXPECTED_LANGUAGES)
        check_after = runner.build_plan(ROOT, "check", EXPECTED_LANGUAGES)

        self.assertTrue(all(step.read_only for step in check_before + check_after))
        self.assertEqual(fix_once, fix_twice)

    def test_shell_plan_retains_each_shebang_dialect(self) -> None:
        runner = load_runner()

        plan = runner.build_plan(ROOT, "check", {"shell"})
        bash = next(
            step for step in plan if "scripts/reconcile-version-pr" in step.command
        )

        self.assertIn("--shell=bash", bash.command)

    def test_inventory_rejects_tracked_symlinks(self) -> None:
        runner = load_runner()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "script.sh").write_text("#!/bin/sh\n", encoding="utf-8")
            os.symlink("script.sh", root / "linked.sh")
            subprocess.run(["git", "init", "--quiet"], cwd=root, check=True)
            subprocess.run(
                ["git", "add", "script.sh", "linked.sh"], cwd=root, check=True
            )

            with self.assertRaises(ValueError):
                runner.tracked_regular_files(root, "*.sh")

    def test_tool_version_policy_is_loaded_from_the_shared_inventory(self) -> None:
        runner = load_runner()

        versions = runner.tool_versions(ROOT)

        self.assertEqual(versions["ruff"], "0.15.2")
        self.assertEqual(versions["prettier"], "3.8.3")

    def test_version_matching_rejects_prefix_collisions(self) -> None:
        runner = load_runner()

        self.assertTrue(runner.version_matches("ruff 0.15.2", "0.15.2"))
        for output in ("ruff 0.15.20", "ruff 0.15.2rc1", "rustc 1.85.0-nightly"):
            with self.subTest(output=output):
                self.assertFalse(runner.version_matches(output, "0.15.2"))

    def test_version_verification_maps_rustc_to_the_rust_policy(self) -> None:
        runner = load_runner()
        plan = [runner.Step("rust", ("cargo",), True)]

        with mock.patch(
            "lint_repository_plan.subprocess.run",
            return_value=SimpleNamespace(stdout="rustc 1.85.0\n"),
        ):
            self.assertTrue(runner.verify_versions(ROOT, plan))

    def test_shebang_parser_accepts_arguments_and_future_python_versions(self) -> None:
        runner = load_runner()

        for shebang, language in (
            (b"#!/usr/bin/python3 -u", "python"),
            (b"#!/bin/bash -e", "shell"),
            (b"#!/usr/bin/env -S bash -e", "shell"),
            (b"#!/usr/bin/env python3.14", "python"),
        ):
            with self.subTest(shebang=shebang):
                self.assertEqual(runner.shebang_language(shebang), language)

    def test_source_inventory_has_a_tracked_file_for_every_language(self) -> None:
        runner = load_runner()

        for language in runner.LANGUAGES:
            with self.subTest(language=language):
                self.assertTrue(runner.inventory_files(ROOT, language))

        python_files = set(runner.inventory_files(ROOT, "python"))
        self.assertIn(
            "plugins/codexy-github/skills/git-workflow/scripts/bootstrap-codexy-github-agent",
            python_files,
        )
        self.assertIn(
            "plugins/codexy/skills/orchestration/scripts/register-codexy-agents",
            python_files,
        )

    def test_failed_linter_route_cannot_report_success(self) -> None:
        runner = load_runner()

        with mock.patch("lint_repository_plan.verify_versions", return_value=True):
            for language in EXPECTED_LANGUAGES:
                with self.subTest(language=language):
                    route = runner.build_plan(ROOT, "check", {language})
                    failed = runner.Step(
                        language, (sys.executable, "-c", "raise SystemExit(1)"), True
                    )
                    self.assertEqual(runner.run([failed, *route], ROOT), 1)

    def test_unclassified_tracked_shebang_is_rejected(self) -> None:
        runner = load_runner()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "unknown").write_text("#!/bin/unknown\\n", encoding="utf-8")
            subprocess.run(["git", "init", "--quiet"], cwd=root, check=True)
            subprocess.run(["git", "add", "unknown"], cwd=root, check=True)

            with self.assertRaises(ValueError):
                runner.shebang_inventory(root)

    def test_workflow_has_a_job_for_every_inventory_language(self) -> None:
        workflow = (ROOT / ".github/workflows/language-lint.yml").read_text(
            encoding="utf-8"
        )

        for language in EXPECTED_LANGUAGES:
            with self.subTest(language=language):
                self.assertIn(f"--language {language}", workflow)

        self.assertIn("name: Language lint", workflow)
        self.assertIn("needs: [rust, python, text, shell, windows]", workflow)
        for job in ("rust", "python", "text", "shell", "windows"):
            self.assertIn(f"test '${{{{ needs.{job}.result }}}}' = success", workflow)
        self.assertIn("$RUNNER_TEMP/codexy-lint-tools", workflow)
        self.assertIn("Get-FileHash -Algorithm SHA256", workflow)
        self.assertIn("psScriptAnalyzerNupkgSha256", workflow)
        for action in (
            "actions/checkout@v7",
            "actions/setup-python@v7",
            "actions/setup-node@v5",
        ):
            with self.subTest(action=action):
                self.assertIn(action, workflow)


if __name__ == "__main__":
    unittest.main()
