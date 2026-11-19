"""Exercise first-run and lost-state distinctions without an installed app."""

import os
import json
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

HOOKS = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"
from packages.getcodexy.tests.recipient_test_support import (
    Dispatch,
    open_store,
    recipient_store,
)

resolve = recipient_store.resolve


def resolve_ordinary(root: str) -> str:
    """Run initialization in an independent process like a host hook."""
    return resolve(root, "ordinary", "other").status


class RecipientStoreInitializationTests(unittest.TestCase):
    @unittest.skipUnless(os.name != "nt", "POSIX link semantics")
    def test_linked_receipts_are_never_trusted(self) -> None:
        for kind in ("symlink", "hardlink"):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as root:
                os.chmod(root, 0o700)
                target = Path(root) / "other-file"
                target.write_text("v1\n")
                target.chmod(0o600)
                receipt = Path(root) / "recipient-store-initialized"
                if kind == "symlink":
                    receipt.symlink_to(target)
                else:
                    os.link(target, receipt)
                self.assertEqual(
                    resolve(root, "ordinary", "other").status, "unavailable"
                )
                self.assertEqual(target.read_text(), "v1\n")

    def test_fresh_ordinary_delivery_runs_through_hook(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            os.chmod(root, 0o700)
            payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "codex_app__send_message_to_thread",
                "session_id": "ordinary",
                "tool_input": {
                    "threadId": "other",
                    "model": "gpt-6.2-sol",
                    "thinking": "medium",
                },
            }
            result = subprocess.run(
                [
                    sys.executable,
                    str(HOOKS / "codexy-thread-delivery.py"),
                    "--event",
                    "PreToolUse",
                ],
                input=json.dumps(payload),
                text=True,
                capture_output=True,
                env={**os.environ, "PLUGIN_DATA": root},
                check=True,
            )
            self.assertEqual(result.stdout, "")

    def test_partial_existing_schema_is_never_repaired_as_empty(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            os.chmod(root, 0o700)
            store = open_store(root)
            Dispatch(store).bootstrap("parent", [])
            database = Path(root) / "supervision/reservations.sqlite3"
            with sqlite3.connect(database) as db:
                db.execute("DROP TABLE reservations")
            self.assertEqual(resolve(root, "ordinary", "other").status, "unavailable")
            with sqlite3.connect(database) as db:
                row = db.execute(
                    "SELECT name FROM sqlite_master WHERE name='reservations'"
                ).fetchone()
            self.assertIsNone(row)

    def test_first_run_allows_ordinary_but_missing_database_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            os.chmod(root, 0o700)
            self.assertEqual(resolve(root, "ordinary", "other").status, "unrelated")
            database = Path(root) / "supervision/reservations.sqlite3"
            database.unlink()
            self.assertEqual(resolve(root, "ordinary", "other").status, "unavailable")
            self.assertFalse(database.exists())

    def test_existing_valid_store_is_preserved_and_corruption_denied(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            os.chmod(root, 0o700)
            store = open_store(root)
            Dispatch(store).bootstrap(
                "parent",
                [
                    {
                        "request": "req",
                        "issue": "issue",
                        "model": "gpt-6.1-terra",
                        "thinking": "medium",
                        "child": "child",
                    }
                ],
            )
            self.assertEqual(resolve(root, "parent", "child").model, "gpt-6.1-terra")
            database = Path(root) / "supervision/reservations.sqlite3"
            database.write_bytes(b"corrupted")
            self.assertEqual(resolve(root, "ordinary", "other").status, "unavailable")

    def test_interrupted_receipt_and_concurrent_initialization(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            os.chmod(root, 0o700)
            (Path(root) / "recipient-store-initialized").write_text("v1\n")
            if os.name != "nt":
                (Path(root) / "recipient-store-initialized").chmod(0o600)
            self.assertEqual(resolve(root, "ordinary", "other").status, "unavailable")
        with tempfile.TemporaryDirectory() as root:
            os.chmod(root, 0o700)
            with ProcessPoolExecutor(max_workers=8) as pool:
                results = list(pool.map(resolve_ordinary, [root] * 8))
            self.assertTrue(set(results) <= {"unrelated", "unavailable"})
            self.assertEqual(resolve(root, "ordinary", "other").status, "unrelated")
