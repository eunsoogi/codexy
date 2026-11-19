"""Check private reservation storage and fail-closed path handling."""

import os
import tempfile
import unittest
from pathlib import Path

from packages.getcodexy.tests.supervision_test_support import reservations, storage

ReservationError, open_store = reservations.ReservationError, storage.open_store


class StorageTests(unittest.TestCase):
    def test_private_store_persists_and_rejects_relative_paths(self):
        with tempfile.TemporaryDirectory() as root:
            store = open_store(root)
            self.assertEqual(store.reserve("p", "r", "i", "m", "t", 1), "reserved")
            self.assertEqual(
                open_store(root).reserve("p", "r", "i", "m", "t", 1), "existing"
            )
            with self.assertRaises(ReservationError):
                open_store("relative")

    @unittest.skipIf(os.name == "nt", "POSIX permissions and symlinks")
    def test_shared_root_and_symlink_are_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            os.chmod(root, 0o755)
            with self.assertRaises(ReservationError):
                open_store(root)
            os.chmod(root, 0o700)
            os.symlink(root, Path(root) / "supervision")
            with self.assertRaises(ReservationError):
                open_store(root)

    @unittest.skipIf(os.name == "nt", "POSIX link handling")
    def test_database_symlink_is_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            directory = Path(root) / "supervision"
            directory.mkdir(mode=0o700)
            target = Path(root) / "other"
            target.write_text("preserve")
            (directory / "reservations.sqlite3").symlink_to(target)
            with self.assertRaises(ReservationError):
                open_store(root)
            self.assertEqual(target.read_text(), "preserve")
