import hashlib
import io
import os
import json
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from codexy_runtime_tools import package, runtime
from codexy_runtime_tools.installer import execute, install_git, install_package
from codexy_runtime_tools.package import _github_token_for


class RuntimeCliTests(unittest.TestCase):
    def test_distribution_identity_and_console_entrypoint_are_stable(self) -> None:
        pyproject = Path(__file__).parents[1].joinpath("pyproject.toml").read_text()
        self.assertIn('name = "getcodexy"', pyproject)
        self.assertRegex(pyproject, r'version = "[0-9]+\.[0-9]+\.[0-9]+"')
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

    def test_tokenless_download_uses_standard_urlopen(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "package.tar.gz"
            response = mock.MagicMock()
            response.__enter__.return_value = io.BytesIO(b"package")
            with (
                mock.patch("codexy_runtime_tools.package.urllib.request.urlopen", return_value=response) as urlopen,
                mock.patch("codexy_runtime_tools.package.urllib.request.build_opener") as build_opener,
            ):
                package._download("https://downloads.example.test/package.tar.gz", destination)
            build_opener.assert_not_called()
            request = urlopen.call_args.args[0]
            self.assertEqual(request.full_url, "https://downloads.example.test/package.tar.gz")
            self.assertIsNone(request.get_header("Authorization"))
            self.assertEqual(destination.read_bytes(), b"package")

    def test_git_repair_forces_install_and_preserves_plugin_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            install_root, installed = root / "cache", root / "cache/bin/codexy-mcp-lsp"
            manifest = root / "plugin.json"
            manifest.write_text('{"version":"1.2.1"}', encoding="utf-8")
            config = SimpleNamespace(server="lsp", manifest=manifest, git_repository="https://github.com/eunsoogi/codexy", git_ref="a" * 40)

            def cargo_install(command: list[str], **_: object) -> SimpleNamespace:
                staged_root = Path(command[command.index("--root") + 1])
                staged = staged_root / "bin" / "codexy-mcp-lsp"
                staged.parent.mkdir(parents=True)
                staged.write_text("#!/bin/sh\n", encoding="utf-8")
                staged.chmod(0o755)
                return SimpleNamespace(returncode=0)

            with (
                mock.patch("codexy_runtime_tools.installer.shutil.which", return_value="/cargo"),
                mock.patch("codexy_runtime_tools.installer.subprocess.run", side_effect=cargo_install) as cargo,
                mock.patch.dict(os.environ, {"GH_TOKEN": "secret", "GITHUB_TOKEN": "secret"}, clear=True),
            ):
                install_git(config, install_root, installed)
            self.assertIn("--force", cargo.call_args.args[0])
            self.assertNotIn("GH_TOKEN", cargo.call_args.kwargs["env"])
            self.assertNotIn("GITHUB_TOKEN", cargo.call_args.kwargs["env"])
            self.assertEqual((install_root / "plugin.json").read_text(encoding="utf-8"), manifest.read_text(encoding="utf-8"))
        with (
            mock.patch.dict("os.environ", {"PRESERVED": "yes"}, clear=True),
            mock.patch("codexy_runtime_tools.installer.os.execvpe") as execvpe,
            self.assertRaisesRegex(AssertionError, "exec returned unexpectedly"),
        ):
            execute("/runtime", ["--stdio"], {"CODEXY_PLUGIN_ROOT": "/installed/plugin"})
        execvpe.assert_called_once_with("/runtime", ["/runtime", "--stdio"], {"PRESERVED": "yes", "CODEXY_PLUGIN_ROOT": "/installed/plugin"})

    def test_package_install_copies_verified_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "package.tar.gz"
            runtime_name = "codexy-mcp-lsp-linux-x86_64.bin"
            with tarfile.open(archive, "w:gz") as packaged:
                for name, contents, mode in (
                    (f"plugins/codexy/runtime/{runtime_name}", b"#!/bin/sh\n", 0o755),
                    ("plugins/codexy/.codex-plugin/plugin.json", b'{"version":"1.2.1"}', 0o644),
                ):
                    member = tarfile.TarInfo(name)
                    member.size, member.mode = len(contents), mode
                    packaged.addfile(member, io.BytesIO(contents))
            manifest = root / "plugin.json"
            manifest.write_text('{"version":"1.2.1"}', encoding="utf-8")
            config = SimpleNamespace(
                package_path=str(archive), package_url="", artifacts_api="",
                package_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                package_override=True, runtime_name=runtime_name, manifest=manifest,
            )
            installed = root / "cache/bin/codexy-mcp-lsp"
            install_package(config, root / "cache", installed)
            self.assertEqual(installed.read_bytes(), b"#!/bin/sh\n")
            self.assertTrue(installed.stat().st_mode & 0o111)


if __name__ == "__main__":
    unittest.main()
