import json
import re
import shutil
import tomllib
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from codexy_runtime_tools.component_integrity import verify_component
from codexy_runtime_tools import updater
from codexy_runtime_tools.github_pre_session import run_github_pre_session
from codexy_runtime_tools.pre_session import run_pre_session


class PublicActivationContractTests(unittest.TestCase):
    def test_github_component_has_a_public_dependency_aware_activation_command(
        self,
    ) -> None:
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
            "test_component_lifecycle_version_admission.py",
            "test_component_lifecycle_interrupt.py",
            "test_component_lifecycle_journal.py",
            "test_component_transition_model.py",
            "test_component_lifecycle_finalization.py",
            "test_component_lifecycle_preflight.py",
            "test_component_lifecycle_update_recovery.py",
            "test_component_lifecycle_admission.py",
            "test_component_transaction_durability.py",
        ):
            self.assertIn(test, workflow)
        self.assertNotIn("-p 'test_component*.py'", workflow)
        self.assertNotIn("test_component_integrity_windows.py", workflow)
        self.assertNotIn("test_component_manifest_resolver.py", workflow)
        self.assertIn("& $candidatePackage --help", workflow)
        self.assertIn(
            'Join-Path $candidateScripts "codexy-github-install.exe") --help',
            workflow,
        )
        self.assertIn("test_version_lock.py", workflow)
        self.assertIn("default_package_version", workflow)
        self.assertIn(
            '$env:PYTHONPATH = "packages/getcodexy/tests"\n'
            "          & $candidatePython -m unittest "
            "packages/getcodexy/tests/test_component_distribution.py",
            workflow,
        )
        self.assertIn(
            'Join-Path $candidateScripts "codexy-github-check.exe") --check-pr-labels',
            workflow,
        )
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

        self.assertNotIn("codexy-update", scripts)
        for path in (
            repository / "README.md",
            repository / "README.ko.md",
            repository / ".github/workflows/python-package.yml",
            repository / "packages/getcodexy/src/codexy_runtime_tools/runtime.py",
            repository / "plugins/codexy/check-codexy-agents",
            repository
            / "plugins/codexy/skills/orchestration/references/agent-registration.md",
        ):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("chmod +x install && ./install", text)
            self.assertNotIn("run the root installer", text)
            self.assertNotIn("codexy-update", text)
            self.assertIsNone(activation_pattern.search(text))

        self.assertTrue(callable(updater.main))
        self.assertTrue(callable(run_pre_session))

    def test_ordinary_source_edits_do_not_require_repinning(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        edits = {
            "codexy": "skills/wiki/SKILL.md",
            "codexy-github": "skills/git-workflow/SKILL.md",
        }
        for component, relative in edits.items():
            with self.subTest(component=component), TemporaryDirectory() as temporary:
                plugin = Path(temporary) / component
                shutil.copytree(repository / "plugins" / component, plugin)
                source = plugin / relative
                source.write_bytes(source.read_bytes() + b"\n# ordinary source edit\n")
                data = json.loads((plugin / ".codex-plugin/plugin.json").read_bytes())
                verified = verify_component(plugin, component, data["version"])
                self.assertEqual(verified[Path(relative)], source.read_bytes())

    def test_package_lifecycle_supplies_prepublication_wheel_and_runtime_inputs(
        self,
    ) -> None:
        repository = Path(__file__).resolve().parents[3]
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        build = workflow.index("      - name: Build distribution")
        wheels = workflow.index("      - name: Prepare MCP wheel fixtures")
        focused = workflow.index("      - name: Run focused package tests")
        self.assertLess(build, wheels)
        self.assertLess(wheels, focused)
        self.assertIn(
            'UV_NO_INDEX=1 UV_FIND_LINKS="$CODEXY_SELECTED_MCP_WHEEL_DIR"',
            workflow,
        )
        fixture_helper = (
            repository / ".github/scripts/prepare-mcp-wheel-fixtures.ps1"
        ).read_text(encoding="utf-8")
        self.assertIn(
            "bash scripts/download-selected-runtime-package.sh dist/selected-runtime-package.tar.gz",
            fixture_helper,
        )
        for field in (
            "CODEXY_RUNTIME_CORE_DIR",
            "CODEXY_RUNTIME_DEVTOOLS_DIR",
            "runtime_source_commit",
            "runtime_archive_version",
            "_safe_extract_tar",
        ):
            self.assertIn(field, fixture_helper)
        smoke = (repository / ".github/scripts/smoke-registered-mcp.ps1").read_text(
            encoding="utf-8"
        )
        self.assertIn('"CORE"', smoke)
        self.assertIn('"DEVTOOLS"', smoke)
        self.assertIn("CODEXY_RUNTIME_${runtimePrefix}_DIR", smoke)


if __name__ == "__main__":
    unittest.main()
