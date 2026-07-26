from __future__ import annotations

import re
import tomllib
import unittest
from pathlib import Path

from codexy_runtime_tools import updater
from codexy_runtime_tools.pre_session import run_pre_session


class PublicActivationContractTests(unittest.TestCase):
    def test_source_only_updater_has_no_public_activation_contract(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        package_metadata = (repository / "packages/getcodexy/pyproject.toml").read_text(
            encoding="utf-8"
        )
        scripts = tomllib.loads(package_metadata)["project"]["scripts"]
        activation_pattern = re.compile(
            r"\buvx\s+--from\s+getcodexy(?:==[^\s]+)?\s+codexy-update\s+--pre-session\b"
        )
        activation_commands = (
            "uvx --from getcodexy codexy-update --pre-session",
            "uvx --from getcodexy==1.2.2 codexy-update --pre-session",
        )
        metadata_with_non_scripts = """[project]
description = "codexy-update remains source-only"
[project.scripts]
codexy-mcp-runtime = "codexy_runtime_tools.runtime:main"
# codexy-update is not an entry point
[tool.example]
codexy-update = "unrelated metadata"
"""
        metadata_with_public_script = """[project]
[project.scripts]
codexy-update = "codexy_runtime_tools.updater:main"
"""

        self.assertFalse((repository / "install").exists())
        self.assertEqual(
            scripts,
            {"codexy-mcp-runtime": "codexy_runtime_tools.runtime:main"},
        )
        self.assertEqual(
            tomllib.loads(metadata_with_non_scripts)["project"]["scripts"],
            scripts,
        )
        self.assertIn(
            "codexy-update",
            tomllib.loads(metadata_with_public_script)["project"]["scripts"],
        )
        self.assertIsNone(activation_pattern.search("unrelated codexy-update text"))
        for command in activation_commands:
            self.assertEqual(activation_pattern.search(command).group(), command)

        for path in (
            repository / "README.md",
            repository / "README.ko.md",
            repository / ".github/workflows/python-package.yml",
            repository / "packages/getcodexy/src/codexy_runtime_tools/runtime.py",
            repository / "plugins/codexy/check-codexy-agents",
            repository / "plugins/codexy/skills/codex-orchestration/references/agent-registration.md",
        ):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("chmod +x install && ./install", text)
            self.assertNotIn("run the root installer", text)
            self.assertNotIn("codexy-update", text)
            self.assertIsNone(activation_pattern.search(text))

        self.assertTrue(callable(updater.main))
        self.assertTrue(callable(run_pre_session))
