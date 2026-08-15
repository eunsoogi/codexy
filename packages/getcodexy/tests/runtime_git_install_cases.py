"""Git runtime installer publication cases."""

import os
import subprocess
import tempfile
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from codexy_runtime_tools import runtime
from codexy_runtime_tools.installer import execute, install_git


class RuntimeGitInstallCases:
    def test_git_repair_is_atomic_tokenless_and_does_not_stamp_a_release_marker(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            install_root, installed = root / "cache", root / "cache/bin/codexy-mcp-lsp"
            manifest = root / "plugin.json"
            manifest.write_text('{"version":"1.2.1"}', encoding="utf-8")
            config = SimpleNamespace(
                server="lsp",
                manifest=manifest,
                git_repository="https://github.com/eunsoogi/codexy",
                git_ref="a" * 40,
            )

            def cargo_install(command: list[str], **_: object) -> SimpleNamespace:
                staged_root = Path(command[command.index("--root") + 1])
                staged = staged_root / "bin" / "codexy-mcp-lsp"
                staged.parent.mkdir(parents=True)
                staged.write_text(
                    "#!/bin/sh\nprintf 'runtime-ok\\n'\n", encoding="utf-8"
                )
                staged.chmod(0o755)
                return SimpleNamespace(returncode=0)

            with (
                mock.patch(
                    "codexy_runtime_tools.installer.shutil.which", return_value="/cargo"
                ),
                mock.patch(
                    "codexy_runtime_tools.installer.subprocess.run",
                    side_effect=cargo_install,
                ) as cargo,
                mock.patch.dict(
                    os.environ,
                    {"GH_TOKEN": "secret", "GITHUB_TOKEN": "secret"},
                    clear=True,
                ),
            ):
                install_git(config, install_root, installed)
            command = cargo.call_args.args[0]
            staged_root = Path(command[command.index("--root") + 1])
            self.assertEqual(
                command,
                [
                    "/cargo",
                    "install",
                    "--force",
                    "--locked",
                    "--git",
                    "https://github.com/eunsoogi/codexy",
                    "--rev",
                    "a" * 40,
                    "--root",
                    str(staged_root),
                    "--bin",
                    "codexy-mcp-lsp",
                    "codexy-runtime",
                ],
            )
            completed = subprocess.run(
                [str(installed)], check=False, text=True, capture_output=True
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(completed.stdout, "runtime-ok\n")
            self.assertEqual(installed.stat().st_mode & 0o777, 0o755)
            self.assertNotIn("GH_TOKEN", cargo.call_args.kwargs["env"])
            self.assertNotIn("GITHUB_TOKEN", cargo.call_args.kwargs["env"])
            self.assertFalse((install_root / "plugin.json").exists())
        with (
            mock.patch.dict("os.environ", {"PRESERVED": "yes"}, clear=True),
            mock.patch("codexy_runtime_tools.installer.os.execvpe") as execvpe,
            self.assertRaisesRegex(AssertionError, "exec returned unexpectedly"),
        ):
            execute(
                "/runtime", ["--stdio"], {"CODEXY_PLUGIN_ROOT": "/installed/plugin"}
            )
        execvpe.assert_called_once_with(
            "/runtime",
            ["/runtime", "--stdio"],
            {"PRESERVED": "yes", "CODEXY_PLUGIN_ROOT": "/installed/plugin"},
        )

    def test_failed_git_install_never_publishes_its_staged_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            install_root = root / "cache"
            installed = install_root / "bin/codexy-mcp-lsp"
            manifest = root / "plugin.json"
            manifest.write_text('{"version":"1.2.2"}', encoding="utf-8")
            config = SimpleNamespace(
                server="lsp",
                manifest=manifest,
                git_repository=runtime.REPOSITORY,
                git_ref="a" * 40,
            )

            def partial_install(command: list[str], **_: object) -> SimpleNamespace:
                staged_root = Path(command[command.index("--root") + 1])
                staged = staged_root / "bin/codexy-mcp-lsp"
                staged.parent.mkdir(parents=True)
                staged.write_text("partial", encoding="utf-8")
                staged.chmod(0o755)
                return SimpleNamespace(returncode=1)

            with (
                mock.patch(
                    "codexy_runtime_tools.installer.shutil.which", return_value="/cargo"
                ),
                mock.patch(
                    "codexy_runtime_tools.installer.subprocess.run",
                    side_effect=partial_install,
                ),
                self.assertRaisesRegex(RuntimeError, "status 1"),
            ):
                install_git(config, install_root, installed)
            self.assertFalse(installed.exists())

    def test_git_install_rejects_a_non_executable_staged_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            installed = root / "cache/bin/codexy-mcp-lsp"
            config = SimpleNamespace(
                server="lsp",
                manifest=root / "plugin.json",
                git_repository=runtime.REPOSITORY,
                git_ref="a" * 40,
            )

            def non_executable_install(
                command: list[str], **_: object
            ) -> SimpleNamespace:
                staged_root = Path(command[command.index("--root") + 1])
                staged = staged_root / "bin/codexy-mcp-lsp"
                staged.parent.mkdir(parents=True)
                staged.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
                staged.chmod(0o644)
                return SimpleNamespace(returncode=0)

            with (
                mock.patch(
                    "codexy_runtime_tools.installer.shutil.which", return_value="/cargo"
                ),
                mock.patch(
                    "codexy_runtime_tools.installer.subprocess.run",
                    side_effect=non_executable_install,
                ),
                self.assertRaisesRegex(RuntimeError, "status 0"),
            ):
                install_git(config, root / "cache", installed)
            self.assertFalse(installed.exists())

    def test_git_install_copy_and_replace_failures_leave_no_partial_temp(self) -> None:
        for failure in ("copy", "replace"):
            with (
                self.subTest(failure=failure),
                tempfile.TemporaryDirectory() as temporary,
            ):
                root = Path(temporary)
                install_root = root / "cache"
                installed = install_root / "bin/codexy-mcp-lsp"
                config = SimpleNamespace(
                    server="lsp",
                    manifest=root / "plugin.json",
                    git_repository=runtime.REPOSITORY,
                    git_ref="a" * 40,
                )

                def cargo_install(command: list[str], **_: object) -> SimpleNamespace:
                    staged_root = Path(command[command.index("--root") + 1])
                    staged = staged_root / "bin/codexy-mcp-lsp"
                    staged.parent.mkdir(parents=True)
                    staged.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
                    staged.chmod(0o755)
                    return SimpleNamespace(returncode=0)

                failure_patch = (
                    mock.patch(
                        "codexy_runtime_tools.installer.shutil.copyfile",
                        side_effect=OSError("copy failed"),
                    )
                    if failure == "copy"
                    else mock.patch(
                        "codexy_runtime_tools.installer.os.replace",
                        side_effect=OSError("replace failed"),
                    )
                )
                with (
                    mock.patch(
                        "codexy_runtime_tools.installer.shutil.which",
                        return_value="/cargo",
                    ),
                    mock.patch(
                        "codexy_runtime_tools.installer.subprocess.run",
                        side_effect=cargo_install,
                    ),
                    failure_patch,
                    self.assertRaisesRegex(OSError, f"{failure} failed"),
                ):
                    install_git(config, install_root, installed)
                self.assertFalse(installed.exists())
                self.assertEqual(
                    list(installed.parent.glob(f".{installed.name}.*.tmp")), []
                )
