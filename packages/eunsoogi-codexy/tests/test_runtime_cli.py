import sys
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import runtime


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


if __name__ == "__main__":
    unittest.main()
