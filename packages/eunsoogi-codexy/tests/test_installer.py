import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from codexy_runtime_tools.installer import execute, install_git


class GitInstallerTests(unittest.TestCase):
    def test_execute_adds_plugin_root_to_runtime_environment(self) -> None:
        with (
            mock.patch.dict("os.environ", {"PRESERVED": "yes"}, clear=True),
            mock.patch("codexy_runtime_tools.installer.os.execvpe") as execvpe,
            self.assertRaisesRegex(AssertionError, "exec returned unexpectedly"),
        ):
            execute(
                "/runtime",
                ["--stdio"],
                {"CODEXY_PLUGIN_ROOT": "/installed/plugin"},
            )

        execvpe.assert_called_once_with(
            "/runtime",
            ["/runtime", "--stdio"],
            {"PRESERVED": "yes", "CODEXY_PLUGIN_ROOT": "/installed/plugin"},
        )

    def test_successful_git_install_records_reusable_release_marker(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            install_root = root / "cache"
            installed = install_root / "bin" / "codexy-mcp-lsp"
            manifest = root / "plugin.json"
            manifest.write_text(json.dumps({"version": "1.2.1"}), encoding="utf-8")
            config = SimpleNamespace(
                server="lsp",
                manifest=manifest,
                git_repository="https://example.test/codexy.git",
                git_ref="a" * 40,
            )

            def cargo_install(*_: object, **__: object) -> SimpleNamespace:
                installed.parent.mkdir(parents=True)
                installed.write_text("#!/bin/sh\n", encoding="utf-8")
                installed.chmod(0o755)
                return SimpleNamespace(returncode=0)

            with (
                mock.patch("codexy_runtime_tools.installer.shutil.which", return_value="/cargo"),
                mock.patch(
                    "codexy_runtime_tools.installer.subprocess.run", side_effect=cargo_install
                ) as cargo,
            ):
                install_git(config, install_root, installed)

            self.assertIn("--force", cargo.call_args.args[0])
            marker = install_root / "plugin.json"
            self.assertEqual(marker.read_text(encoding="utf-8"), manifest.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
