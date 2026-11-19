"""Distinguish a fresh delivery registry from loss within a retained data root."""

from contextlib import closing
import os
from pathlib import Path
import sqlite3

from .supervision_dispatch import Dispatch
from .supervision_reservations import ReservationError, Reservations
from .supervision_storage import _private, open_store

RECEIPT = "recipient-store-initialized"
CONTENTS = b"v1\n"


def _receipt(path: Path, *, create: bool) -> bool:
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    if create:
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags, 0o600)
    except FileExistsError:
        return False
    try:
        info = os.fstat(descriptor)
        if (
            not _private(info, directory=False)
            or info.st_nlink != 1
            or path.is_symlink()
        ):
            raise ReservationError("recipient initialization receipt is not private")
        if create:
            if os.write(descriptor, CONTENTS) != len(CONTENTS):
                raise ReservationError("recipient initialization receipt is incomplete")
            os.fsync(descriptor)
        elif os.read(descriptor, len(CONTENTS) + 1) != CONTENTS:
            raise ReservationError("recipient initialization receipt is invalid")
    finally:
        os.close(descriptor)
    if create and os.name != "nt":
        # Persist the directory entry as well as its contents before creating a DB.
        parent = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(parent)
        finally:
            os.close(parent)
    return True


def _valid_existing(database: Path) -> None:
    """Never manufacture a complete owner roster for pre-existing state."""
    directory_info = database.parent.lstat()
    info = database.lstat()
    if (
        not _private(directory_info, directory=True)
        or not _private(info, directory=False)
        or info.st_nlink != 1
        or info.st_size == 0
    ):
        raise ReservationError("supervision database is not private or is empty")
    # Read before opening the writable store: its constructor creates tables and
    # would otherwise conceal a partially lost registry.
    with closing(
        sqlite3.connect(database.as_uri() + "?mode=ro", uri=True, timeout=2)
    ) as db:
        if db.execute("PRAGMA quick_check").fetchone()[0] != "ok":
            raise ReservationError("supervision database is corrupt")
        tables = {
            row[0]
            for row in db.execute("SELECT name FROM sqlite_master WHERE type='table'")
        }
        if not {"reservations", "supervision_parents"} <= tables:
            raise ReservationError("supervision registry is not initialized")


def open_recipient_store(plugin_data: str) -> Reservations:
    """Create fresh state once; refuse missing DB after a durable receipt exists."""
    root = Path(plugin_data)
    if (
        not plugin_data
        or not root.is_absolute()
        or not _private(root.lstat(), directory=True)
    ):
        raise ReservationError("private plugin data root is unavailable")
    receipt = root / RECEIPT
    directory = root / "supervision"
    database = directory / "reservations.sqlite3"
    if receipt.exists() or receipt.is_symlink():
        _receipt(receipt, create=False)
        if not database.exists() or database.is_symlink():
            raise ReservationError("initialized supervision database is missing")
        _valid_existing(database)
        return open_store(plugin_data)
    if directory.exists() or directory.is_symlink():
        # Existing state must be verified before introducing a receipt; an empty
        # or absent database may be lost supervision data, not a first run.
        if (
            not database.exists()
            or database.is_symlink()
            or database.stat().st_size == 0
        ):
            raise ReservationError("existing supervision database is unavailable")
        _valid_existing(database)
        store = open_store(plugin_data)
        if not _receipt(receipt, create=True):
            _receipt(receipt, create=False)
        return store
    # Persist the receipt first. A crash or racing reader sees incomplete state
    # and denies until recovery instead of treating it as a second first run.
    if not _receipt(receipt, create=True):
        raise ReservationError("recipient initialization is in progress")
    store = open_store(plugin_data)
    Dispatch(store)
    return store
