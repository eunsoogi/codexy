import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from codexy_runtime_tools import runtime
from codexy_runtime_tools.installer import install_git


class GitInstallTests(unittest.TestCase):
    def test_git_install_rejects_wrong_source_or_ref_before_cargo_runs(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for repository, revision in (
                ("https://example.test/not-codexy", "a" * 40),
                (runtime.REPOSITORY, "not-a-commit"),
            ):
                with self.subTest(repository=repository, revision=revision):
                    config = SimpleNamespace(
                        server="lsp",
                        manifest=root / "plugin.json",
                        git_repository=repository,
                        git_ref=revision,
                    )
                    with (
                        mock.patch(
                            "codexy_runtime_tools.installer.subprocess.run"
                        ) as cargo,
                        self.assertRaisesRegex(RuntimeError, "Git fallback requires"),
                    ):
                        install_git(
                            config, root / "cache", root / "cache/bin/codexy-mcp-lsp"
                        )
                    cargo.assert_not_called()

    def test_clean_git_ref_can_install_the_module_owned_runtime_package(self) -> None:
        repository = Path(__file__).parents[3]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "clean-source"
            shutil.copytree(
                repository,
                source,
                ignore=shutil.ignore_patterns(
                    ".git", "target", ".venv", "dist", "uv.lock", "__pycache__", "*.pyc"
                ),
            )
            subprocess.run(["git", "init", "-b", "main"], cwd=source, check=True)
            subprocess.run(
                ["git", "config", "user.name", "test"], cwd=source, check=True
            )
            subprocess.run(
                ["git", "config", "user.email", "test@example.com"],
                cwd=source,
                check=True,
            )
            subprocess.run(["git", "add", "."], cwd=source, check=True)
            subprocess.run(
                ["git", "commit", "-m", "clean module package"], cwd=source, check=True
            )
            revision = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=source, text=True
            ).strip()
            install_root = root / "installed"
            command = [
                "cargo",
                "install",
                "--locked",
                "--git",
                source.as_uri(),
                "--rev",
                revision,
                "--root",
                str(install_root),
                "--bin",
                "codexy-mcp-lsp",
                "codexy-runtime",
            ]
            completed = subprocess.run(
                command, text=True, capture_output=True, timeout=240
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue((install_root / "bin/codexy-mcp-lsp").is_file())
            wrong_package = subprocess.run(
                [*command[:-1], "codexy-runtime-not-a-package"],
                text=True,
                capture_output=True,
                timeout=30,
            )
            self.assertNotEqual(wrong_package.returncode, 0)
            self.assertIn("codexy-runtime-not-a-package", wrong_package.stderr)
