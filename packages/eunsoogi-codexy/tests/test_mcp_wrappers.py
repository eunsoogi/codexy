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


if __name__ == "__main__":
    unittest.main()
