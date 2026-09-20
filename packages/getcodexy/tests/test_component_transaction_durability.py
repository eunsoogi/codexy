from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import nullcontext
from pathlib import Path
from unittest.mock import patch

import codexy_runtime_tools.component_transaction_state as transaction_state
from codexy_runtime_tools.component_transaction_durability import sync_parent_directory
from codexy_runtime_tools.component_transaction_state import (
    capture_inventory_snapshot,
    clear_stale_registration_lock,
    restore_inventory_snapshot,
)


class TransactionDurabilityTests(unittest.TestCase):
    def test_windows_skips_posix_directory_open(self) -> None:
        directory = Path("/temporary/durable-state")
        with (
            patch(
                "codexy_runtime_tools.component_transaction_durability.os.name", "nt"
            ),
            patch(
                "codexy_runtime_tools.component_transaction_durability.os.open"
            ) as opened,
        ):
            sync_parent_directory(directory)
        opened.assert_not_called()

    def test_managed_projection_snapshot_restores_only_managed_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            inventory = home / "getcodexy" / "installed-components.json"
            inventory.parent.mkdir()
            inventory.write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.installed-component-inventory.v1",
                        "components": ["core"],
                    }
                ),
                encoding="utf-8",
            )
            roles = home / "agents" / "codexy"
            roles.mkdir(parents=True)
            role = roles / "codexy-sentinel.toml"
            original_role = b"# CODEXY MANAGED AGENT\nmodel = 'old'\n"
            role.write_bytes(original_role)
            role.chmod(0o640)
            unmanaged = roles / "personal.toml"
            unmanaged.write_bytes(b"personal = true\n")
            config = home / "config.toml"
            original_config = (
                b"[other]\nvalue = true\n"
                b"# BEGIN CODEXY MANAGED AGENTS\n"
                b"[agents.codexy-sentinel]\nmodel = 'old'\n"
                b"# END CODEXY MANAGED AGENTS\n"
            )
            config.write_bytes(original_config)
            backup = home / "config.toml.codexy-backup-old"
            backup.write_bytes(b"old backup\n")

            snapshot = capture_inventory_snapshot(home)
            role.write_bytes(b"# CODEXY MANAGED AGENT\nmodel = 'new'\n")
            (roles / "codexy-new.toml").write_bytes(
                b"# CODEXY MANAGED AGENT\nmodel = 'new'\n"
            )
            config.write_bytes(b"[other]\nvalue = true\n")
            (home / "config.toml.codexy-backup-new").write_bytes(original_config)
            inventory.write_text("changed", encoding="utf-8")
            unmanaged.write_bytes(b"personal = changed\n")

            restore_inventory_snapshot(home, snapshot)

            self.assertEqual(role.read_bytes(), original_role)
            expected_mode = 0o666 if os.name == "nt" else 0o640
            self.assertEqual(role.stat().st_mode & 0o777, expected_mode)
            self.assertFalse((roles / "codexy-new.toml").exists())
            self.assertEqual(config.read_bytes(), original_config)
            self.assertEqual(backup.read_bytes(), b"old backup\n")
            self.assertFalse((home / "config.toml.codexy-backup-new").exists())
            self.assertEqual(unmanaged.read_bytes(), b"personal = changed\n")
            self.assertEqual(
                json.loads(inventory.read_text(encoding="utf-8"))["components"],
                ["core"],
            )

    def test_stale_registration_lock_is_removed_only_after_owner_exit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            lock = home / ".codexy-agent-registration.lock"
            lock.write_text("1234\n", encoding="ascii")
            with patch.object(
                transaction_state, "registration_owner_state", return_value="dead"
            ):
                clear_stale_registration_lock(home)
            self.assertFalse(lock.exists())

            child = subprocess.Popen(
                [sys.executable, "-c", "import time; time.sleep(30)"],
                creationflags=getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0),
                start_new_session=os.name != "nt",
            )
            try:
                lock.write_text(f"{child.pid}\n", encoding="ascii")
                no_signal = (
                    patch.object(
                        transaction_state.os,
                        "kill",
                        side_effect=AssertionError("Windows owner probe signaled"),
                    )
                    if os.name == "nt"
                    else nullcontext()
                )
                with no_signal:
                    with self.assertRaisesRegex(RuntimeError, "registration is active"):
                        clear_stale_registration_lock(home)
                self.assertIsNone(child.poll())
            finally:
                if child.poll() is None:
                    child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=5)

            clear_stale_registration_lock(home)
            self.assertFalse(lock.exists())

    def test_stale_lock_cleanup_preserves_a_replacement_owner(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            lock = home / ".codexy-agent-registration.lock"
            lock.write_text("1234\n", encoding="ascii")
            unlink = transaction_state._unlink_regular
            replacement_owner = "987654\n"

            def replace_owner(path: Path, expected_identity: tuple[int, int]) -> None:
                path.unlink()
                path.write_text(replacement_owner, encoding="ascii")
                unlink(path, expected_identity)

            with (
                patch.object(
                    transaction_state, "registration_owner_state", return_value="dead"
                ),
                patch.object(
                    transaction_state, "_unlink_regular", side_effect=replace_owner
                ),
            ):
                clear_stale_registration_lock(home)

            self.assertEqual(lock.read_text(encoding="ascii"), replacement_owner)

    def test_unobservable_registration_owner_preserves_the_lock(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            lock = home / ".codexy-agent-registration.lock"
            lock.write_text("987654\n", encoding="ascii")
            with patch.object(
                transaction_state, "registration_owner_state", return_value="unknown"
            ):
                with self.assertRaisesRegex(RuntimeError, "active or unobservable"):
                    clear_stale_registration_lock(home)
            self.assertTrue(lock.exists())

    def test_config_snapshot_ignores_literal_markers_in_multiline_strings(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            config = home / "config.toml"
            original = (
                b'notes = """\n'
                b"# BEGIN CODEXY MANAGED AGENTS\n"
                b"# END CODEXY MANAGED AGENTS\n"
                b'"""\n'
                b"# BEGIN CODEXY MANAGED AGENTS\n"
                b"[agents.codexy-sentinel]\nmodel = 'old'\n"
                b"# END CODEXY MANAGED AGENTS\n"
            )
            config.write_bytes(original)
            snapshot = capture_inventory_snapshot(home)
            config.write_bytes(
                b'notes = """\n'
                b"# BEGIN CODEXY MANAGED AGENTS\n"
                b"# END CODEXY MANAGED AGENTS\n"
                b'"""\n'
            )

            restore_inventory_snapshot(home, snapshot)

            self.assertEqual(config.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
