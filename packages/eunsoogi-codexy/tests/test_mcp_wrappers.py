import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


REPOSITORY = Path(__file__).resolve().parents[3]


class McpWrapperTests(unittest.TestCase):
    def wrapper(self, server: str) -> Path:
        return REPOSITORY / "plugins" / "codexy" / "mcp" / f"codexy-mcp-{server}"

    def test_wrapper_fails_visibly_when_uvx_is_unavailable_on_hostile_path(self) -> None:
        completed = subprocess.run(
            [self.wrapper("lsp")],
            env={"PATH": ""},
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 127)
        self.assertIn("requires uvx", completed.stderr)

    def test_wrapper_preserves_unicode_plugin_root_and_stdio_for_pinned_uvx(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "plugin root 유니코드"
            mcp = root / "mcp"
            mcp.mkdir(parents=True)
            wrapper = mcp / "codexy-mcp-lsp"
            shutil.copyfile(self.wrapper("lsp"), wrapper)
            wrapper.chmod(0o755)
            log = root / "received arguments.txt"
            fake_uvx = root / "uvx"
            fake_uvx.write_text('#!/bin/sh\nprintf "%s\\n" "$@" > "$CODEXY_TEST_ARGUMENT_LOG"\n')
            fake_uvx.chmod(0o755)

            completed = subprocess.run(
                [wrapper, "--stdio"],
                env={
                    "PATH": "/hostile path/유니코드",
                    "CODEXY_UVX_PATH": str(fake_uvx),
                    "CODEXY_TEST_ARGUMENT_LOG": str(log),
                },
                capture_output=True,
                text=True,
                check=False,
            )

            self.assertEqual(completed.returncode, 0, completed.stderr)
            arguments = log.read_text(encoding="utf-8").splitlines()
            self.assertIn("eunsoogi-codexy==1.2.1", arguments)
            self.assertEqual(arguments[-4:], ["--plugin-root", str(root), "--", "--stdio"])

    def test_wrapper_runs_bundled_runtime_before_uvx(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            uname = bin_dir / "uname"
            uname.write_text('#!/bin/sh\n[ "$1" = "-s" ] && echo Darwin || echo arm64\n')
            uname.chmod(0o755)
            for server in ("lsp", "codegraph"):
                mcp = root / "mcp"
                mcp.mkdir(exist_ok=True)
                wrapper = mcp / f"codexy-mcp-{server}"
                shutil.copyfile(self.wrapper(server), wrapper)
                wrapper.chmod(0o755)
                runtime = root / "runtime" / f"codexy-mcp-{server}-darwin-arm64.bin"
                runtime.parent.mkdir(exist_ok=True)
                runtime.write_text('#!/bin/sh\nprintf "%s\\n" "$CODEXY_PLUGIN_ROOT" "$@"\n')
                runtime.chmod(0o755)

                completed = subprocess.run(
                    [wrapper, "--stdio"], env={"PATH": str(bin_dir)}, capture_output=True,
                    text=True, check=False,
                )

                self.assertEqual(completed.returncode, 0, completed.stderr)
                self.assertEqual(completed.stdout.splitlines(), [str(root), "--stdio"])

    def test_wrapper_routes_runtime_overrides_through_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            uname = bin_dir / "uname"
            uname.write_text('#!/bin/sh\n[ "$1" = "-s" ] && echo Darwin || echo arm64\n')
            uname.chmod(0o755)
            uvx_log = root / "uvx.log"
            fake_uvx = root / "uvx"
            fake_uvx.write_text('#!/bin/sh\nprintf "%s\\n" "$@" > "$CODEXY_TEST_UVX_LOG"\n')
            fake_uvx.chmod(0o755)
            for environment in (
                {"CODEXY_RUNTIME_PLATFORM": "linux-x86_64"},
            ):
                for server in ("lsp", "codegraph"):
                    mcp = root / "mcp"
                    mcp.mkdir(exist_ok=True)
                    wrapper = mcp / f"codexy-mcp-{server}"
                    shutil.copyfile(self.wrapper(server), wrapper)
                    wrapper.chmod(0o755)
                    bundled = root / "runtime" / f"codexy-mcp-{server}-darwin-arm64.bin"
                    bundled.parent.mkdir(exist_ok=True)
                    bundled.write_text('#!/bin/sh\necho bundled-runtime\n')
                    bundled.chmod(0o755)

                    completed = subprocess.run(
                        [wrapper, "--stdio"],
                        env={
                            "PATH": str(bin_dir),
                            "CODEXY_UVX_PATH": str(fake_uvx),
                            "CODEXY_TEST_UVX_LOG": str(uvx_log),
                            **environment,
                        },
                        capture_output=True,
                        text=True,
                        check=False,
                    )

                    self.assertEqual(completed.returncode, 0, completed.stderr)
                    self.assertIn("eunsoogi-codexy==1.2.1", uvx_log.read_text(encoding="utf-8"))

    def test_runtime_directory_override_executes_without_uvx(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            uname = bin_dir / "uname"
            uname.write_text('#!/bin/sh\n[ "$1" = "-s" ] && echo Linux || echo x86_64\n')
            uname.chmod(0o755)
            for server in ("lsp", "codegraph"):
                mcp = root / "mcp"
                mcp.mkdir(exist_ok=True)
                wrapper = mcp / f"codexy-mcp-{server}"
                shutil.copyfile(self.wrapper(server), wrapper)
                wrapper.chmod(0o755)
                override = root / "override" / f"codexy-mcp-{server}-linux-x86_64.bin"
                override.parent.mkdir(exist_ok=True)
                override.write_text('#!/bin/sh\necho override "$@"\n')
                override.chmod(0o755)
                completed = subprocess.run([wrapper, "--stdio"], env={"PATH": str(bin_dir), "UV_OFFLINE": "1", "CODEXY_RUNTIME_DIR": str(override.parent)}, capture_output=True, text=True)
                self.assertEqual(completed.returncode, 0, completed.stderr)
                self.assertEqual(completed.stdout.strip(), "override --stdio")

    def test_runtime_directory_override_honors_platform_override(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            uname = bin_dir / "uname"
            uname.write_text('#!/bin/sh\n[ "$1" = "-s" ] && echo Darwin || echo arm64\n')
            uname.chmod(0o755)
            for server in ("lsp", "codegraph"):
                mcp = root / "mcp"
                mcp.mkdir(exist_ok=True)
                wrapper = mcp / f"codexy-mcp-{server}"
                shutil.copyfile(self.wrapper(server), wrapper)
                wrapper.chmod(0o755)
                override = root / "override" / f"codexy-mcp-{server}-linux-x86_64.bin"
                override.parent.mkdir(exist_ok=True)
                override.write_text('#!/bin/sh\necho platform-override "$@"\n')
                override.chmod(0o755)
                completed = subprocess.run([wrapper, "--stdio"], env={"PATH": str(bin_dir), "UV_OFFLINE": "1", "CODEXY_RUNTIME_DIR": str(override.parent), "CODEXY_RUNTIME_PLATFORM": "linux-x86_64"}, capture_output=True, text=True)
                self.assertEqual(completed.returncode, 0, completed.stderr)
                self.assertEqual(completed.stdout.strip(), "platform-override --stdio")

    def test_runtime_directory_override_must_be_absolute(self) -> None:
        for server in ("lsp", "codegraph"):
            completed = subprocess.run([self.wrapper(server)], env={"PATH": "", "CODEXY_RUNTIME_DIR": "relative"}, capture_output=True, text=True)
            self.assertEqual(completed.returncode, 127)
            self.assertIn("CODEXY_RUNTIME_DIR must be absolute", completed.stderr)


if __name__ == "__main__":
    unittest.main()
