"""Representative contract tests for the repository lint entry point."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "lint-repository.py"


def runner():
    spec = importlib.util.spec_from_file_location("lint_repository", SCRIPT)
    if spec is None or spec.loader is None:
        raise AssertionError("lint runner is not importable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class LintRepositoryTests(unittest.TestCase):
    def test_inventory_covers_maintained_languages(self) -> None:
        module = runner()
        files = module.inventory(ROOT)
        self.assertEqual(set(files), set(module.LANGUAGES))
        self.assertTrue(all(files.values()))

    def test_fix_plan_is_deterministic_and_check_is_read_only(self) -> None:
        module = runner()
        files = {language: [] for language in module.LANGUAGES}
        files["python"] = ["scripts/lint-repository.py"]
        calls: list[tuple[str, ...]] = []

        def record(_root: Path, *args: str) -> int:
            calls.append(args)
            return 0

        with (
            mock.patch.object(module, "inventory", return_value=files),
            mock.patch.object(module, "command", side_effect=record),
        ):
            self.assertEqual(module.run(ROOT, "check", {"python"}), 0)
            checked = list(calls)
            calls.clear()
            self.assertEqual(module.run(ROOT, "fix", {"python"}), 0)
            fixed_once = list(calls)
            calls.clear()
            self.assertEqual(module.run(ROOT, "fix", {"python"}), 0)

        self.assertIn("--check", checked[0])
        self.assertIn("--fix", fixed_once[0])
        self.assertEqual(fixed_once, calls)

    def test_command_launcher_check_fails_closed(self) -> None:
        module = runner()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "bad.cmd"
            path.write_text("echo unsafe\n", encoding="utf-8")
            self.assertEqual(module.check_cmd(path.parent, [path.name]), 1)

    def test_workflow_uses_a_language_matrix_and_one_entry_point(self) -> None:
        workflow = (ROOT / ".github/workflows/language-lint.yml").read_text()
        self.assertIn("matrix:", workflow)
        self.assertIn("python scripts/lint-repository.py --check --language", workflow)
        for language in runner().LANGUAGES:
            self.assertIn(f"language: {language}", workflow)


if __name__ == "__main__":
    unittest.main()
