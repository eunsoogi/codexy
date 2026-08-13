from __future__ import annotations

import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.component_transaction_durability import sync_parent_directory


class TransactionDurabilityTests(unittest.TestCase):
    def test_windows_skips_posix_directory_open(self) -> None:
        directory = Path("/temporary/durable-state")
        with patch("codexy_runtime_tools.component_transaction_durability.os.name", "nt"), patch("codexy_runtime_tools.component_transaction_durability.os.open") as opened:
            sync_parent_directory(directory)
        opened.assert_not_called()


if __name__ == "__main__":
    unittest.main()
