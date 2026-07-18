import sys
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import runtime


class RuntimeCliTests(unittest.TestCase):
    def test_wrapper_argument_order_keeps_plugin_root_and_stdio(self) -> None:
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
