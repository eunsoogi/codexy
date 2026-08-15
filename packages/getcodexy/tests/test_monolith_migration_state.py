from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.monolith_migration_state import (
    MigrationJournal,
    journal_path,
    read_journal,
    write_journal,
)


class MonolithMigrationStateTests(unittest.TestCase):
    def test_journal_round_trip_preserves_the_supported_selection(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            journal = MigrationJournal.capture(
                home, "1.3.0", "1.4.0", ("core", "devtools")
            )
            write_journal(home, journal)

            self.assertEqual(read_journal(home), journal)

    def test_duplicate_json_key_fails_before_recovery_mutates_the_host(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            path = journal_path(home)
            path.parent.mkdir(parents=True)
            path.write_text(
                '{"schema":"getcodexy.monolith-migration.v1",'
                '"source_version":"1.3.0","source_version":"1.4.0",'
                '"target_version":"1.4.0","selection":["core"],'
                '"phase":"prepared","snapshot":[]}',
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "duplicate keys"):
                read_journal(home)

    def test_invalid_selection_cannot_authorize_recovery_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            path = journal_path(home)
            path.parent.mkdir(parents=True)
            path.write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.monolith-migration.v1",
                        "source_version": "1.3.0",
                        "target_version": "1.4.0",
                        "selection": ["github"],
                        "phase": "prepared",
                        "snapshot": [],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "invalid selection"):
                read_journal(home)

    def test_journal_requires_a_distinct_supported_source(self) -> None:
        journal = {
            "schema": "getcodexy.monolith-migration.v1",
            "source_version": "1.3.0",
            "target_version": "1.3.0",
            "selection": ["core"],
            "phase": "prepared",
            "snapshot": [],
        }
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            path = journal_path(home)
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(journal), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "invalid values"):
                read_journal(home)

    def test_journal_rejects_a_snapshot_outside_migration_roots(self) -> None:
        journal = {
            "schema": "getcodexy.monolith-migration.v1",
            "source_version": "1.3.0",
            "target_version": "1.4.0",
            "selection": ["core"],
            "phase": "prepared",
            "snapshot": [{"path": "outside", "mode": 384, "data": None}],
        }
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            path = journal_path(home)
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(journal), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "unsafe snapshot"):
                read_journal(home)


if __name__ == "__main__":
    unittest.main()
