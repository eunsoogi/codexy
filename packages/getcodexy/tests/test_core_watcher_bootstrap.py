"""The current shared bootstrap rejects retired core Watcher selectors."""

from __future__ import annotations

import io
import sys
import unittest
from unittest.mock import patch

from codexy_runtime_tools import mcp_bootstrap


class CoreWatcherBootstrapTests(unittest.TestCase):
    def test_shared_bootstrap_rejects_watcher_before_runtime_selection(self) -> None:
        with (
            patch.object(sys, "stderr", new_callable=io.StringIO) as stderr,
            patch.object(mcp_bootstrap, "_selected_version") as selected_version,
            patch.object(mcp_bootstrap.shutil, "which") as find_uvx,
        ):
            result = mcp_bootstrap.main(["watcher", "--stdio"])

        self.assertEqual(result, 64)
        self.assertIn("requires lsp or codegraph", stderr.getvalue())
        selected_version.assert_not_called()
        find_uvx.assert_not_called()


if __name__ == "__main__":
    unittest.main()
