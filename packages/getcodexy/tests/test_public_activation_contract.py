from __future__ import annotations

import re
import tomllib
import unittest
from pathlib import Path

from codexy_runtime_tools import updater
from codexy_runtime_tools.github_pre_session import run_github_pre_session
from codexy_runtime_tools.pre_session import run_pre_session


class PublicActivationContractTests(unittest.TestCase):
    def test_github_component_has_a_public_dependency_aware_activation_command(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        metadata = (repository / "packages/getcodexy/pyproject.toml").read_text(
            encoding="utf-8"
        )
        scripts = tomllib.loads(metadata)["project"]["scripts"]

        self.assertEqual(
            scripts["codexy-github-install"],
            "codexy_runtime_tools.github_pre_session:main",
        )
        self.assertEqual(
            scripts["codexy-github-check"],
            "codexy_runtime_tools.github_checks:main",
        )
        self.assertTrue(callable(run_github_pre_session))
        self.assertFalse((repository / "install").exists())
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("github-activation-windows", workflow)
        self.assertIn("Run native component lifecycle tests", workflow)
        for test in (
            "test_component_cli.py",
            "test_component_lifecycle.py",
            "test_component_lifecycle_interrupt.py",
            "test_component_lifecycle_journal.py",
            "test_component_lifecycle_finalization.py",
            "test_component_lifecycle_preflight.py",
            "test_component_lifecycle_update_recovery.py",
            "test_component_transaction_durability.py",
        ):
            self.assertIn(test, workflow)
        self.assertNotIn("-p 'test_component*.py'", workflow)
        self.assertNotIn("test_component_integrity_windows.py", workflow)
        self.assertNotIn("test_component_manifest_resolver.py", workflow)
        self.assertIn("getcodexy.exe --help", workflow)
        self.assertIn("codexy-github-install.exe --help", workflow)
        self.assertIn("codexy-github-check.exe --check-pr-labels", workflow)
        self.assertIn("& (Join-Path $hookRoot", workflow)
        self.assertNotIn("cmd /d /s /c", workflow)
        self.assertIn('"plugins/codexy-github/**"', workflow)

    def test_source_only_updater_remains_unpublished(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        metadata = (repository / "packages/getcodexy/pyproject.toml").read_text(
            encoding="utf-8"
        )
        scripts = tomllib.loads(metadata)["project"]["scripts"]
        activation_pattern = re.compile(
            r"\buvx\s+--from\s+getcodexy(?:==[^\s]+)?\s+codexy-update\s+--pre-session\b"
        )

        self.assertFalse((repository / "install").exists())
        self.assertNotIn("codexy-update", scripts)
        for path in (
            repository / "README.md",
            repository / "README.ko.md",
            repository / ".github/workflows/python-package.yml",
            repository / "packages/getcodexy/src/codexy_runtime_tools/runtime.py",
            repository / "plugins/codexy/check-codexy-agents",
            repository / "plugins/codexy/skills/orchestration/references/agent-registration.md",
        ):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("chmod +x install && ./install", text)
            self.assertNotIn("run the root installer", text)
            self.assertNotIn("codexy-update", text)
            self.assertIsNone(activation_pattern.search(text))

        self.assertTrue(callable(updater.main))
        self.assertTrue(callable(run_pre_session))


if __name__ == "__main__":
    unittest.main()
