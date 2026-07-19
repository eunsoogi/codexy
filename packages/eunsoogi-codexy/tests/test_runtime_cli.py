import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from codexy_runtime_tools import package, runtime
from codexy_runtime_tools.installer import execute, install_git
from codexy_runtime_tools.package import _github_token_for


class RuntimeCliTests(unittest.TestCase):
    def test_distribution_identity_and_console_entrypoint_are_stable(self) -> None:
        pyproject = Path(__file__).parents[1].joinpath("pyproject.toml").read_text()
        self.assertIn('name = "eunsoogi-codexy"', pyproject)
        self.assertIn('version = "1.2.2"', pyproject)
        self.assertIn(
            'codexy-mcp-runtime = "codexy_runtime_tools.runtime:main"', pyproject
        )

    def test_cli_preserves_plugin_root_and_stdio_arguments(self) -> None:
        argv = [
            "codexy-mcp-runtime",
            "lsp",
            "--plugin-root",
            "/tmp/plugin root",
            "--",
            "--stdio",
        ]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(
            runtime.Configuration, "load"
        ) as load, mock.patch.object(runtime, "run"):
            runtime.main()

        load.assert_called_once_with("lsp", Path("/tmp/plugin root").resolve(), ["--stdio"])

    def test_github_token_policy_uses_environment_then_cli_on_trusted_api(self) -> None:
        api = "https://api.github.com/repos/eunsoogi/codexy/actions/artifacts"
        with mock.patch.dict("os.environ", {"GH_TOKEN": "environment-token"}, clear=True), mock.patch.object(package.subprocess, "run") as run:
            self.assertEqual(_github_token_for(api), "environment-token")
        run.assert_not_called()
        completed = package.subprocess.CompletedProcess(["gh"], 0, "cli-token\n", "")
        with mock.patch.dict("os.environ", {}, clear=True), mock.patch.object(package.subprocess, "run", return_value=completed):
            self.assertEqual(_github_token_for(api), "cli-token")
        with mock.patch.dict("os.environ", {}, clear=True), mock.patch.object(package.subprocess, "run") as run:
            self.assertEqual(_github_token_for("https://objects.example.test/artifact.zip"), "")
        run.assert_not_called()

    def test_git_repair_forces_install_and_preserves_plugin_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            install_root, installed = root / "cache", root / "cache/bin/codexy-mcp-lsp"
            manifest = root / "plugin.json"
            manifest.write_text('{"version":"1.2.1"}', encoding="utf-8")
            config = SimpleNamespace(server="lsp", manifest=manifest, git_repository="https://example.test/codexy.git", git_ref="a" * 40)

            def cargo_install(*_: object, **__: object) -> SimpleNamespace:
                installed.parent.mkdir(parents=True)
                installed.write_text("#!/bin/sh\n", encoding="utf-8")
                installed.chmod(0o755)
                return SimpleNamespace(returncode=0)

            with (
                mock.patch("codexy_runtime_tools.installer.shutil.which", return_value="/cargo"),
                mock.patch("codexy_runtime_tools.installer.subprocess.run", side_effect=cargo_install) as cargo,
            ):
                install_git(config, install_root, installed)
            self.assertIn("--force", cargo.call_args.args[0])
            self.assertEqual((install_root / "plugin.json").read_text(encoding="utf-8"), manifest.read_text(encoding="utf-8"))
        with (
            mock.patch.dict("os.environ", {"PRESERVED": "yes"}, clear=True),
            mock.patch("codexy_runtime_tools.installer.os.execvpe") as execvpe,
            self.assertRaisesRegex(AssertionError, "exec returned unexpectedly"),
        ):
            execute("/runtime", ["--stdio"], {"CODEXY_PLUGIN_ROOT": "/installed/plugin"})
        execvpe.assert_called_once_with("/runtime", ["/runtime", "--stdio"], {"PRESERVED": "yes", "CODEXY_PLUGIN_ROOT": "/installed/plugin"})


if __name__ == "__main__":
    unittest.main()
