"""No-follow durable state for component lifecycle transactions."""

from __future__ import annotations

import errno
import json
import os
import stat
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

from .component_transaction_durability import sync_parent_directory
from .component_transaction_snapshot import InventorySnapshot
from .component_transition_model import JOURNAL_SCHEMA, Journal
from .updater import _absolute, _validate_real_path


INVENTORY_SCHEMA = "getcodexy.installed-component-inventory.v1"


class PreAdmissionError(RuntimeError):
    """A lifecycle operation failed before durable state admission."""


def inventory_path(home: str | os.PathLike[str]) -> Path:
    return _absolute(home) / "getcodexy" / "installed-components.json"

def read_inventory(home: Path) -> tuple[str, ...] | None:
    contents = _read_regular(inventory_path(home))
    if contents is None:
        return None
    return decode_inventory(contents)


def decode_inventory(contents: bytes) -> tuple[str, ...]:
    data = json.loads(contents, object_pairs_hook=_unique_object)
    components = data.get("components") if isinstance(data, dict) else None
    if not isinstance(data, dict) or set(data) != {"schema", "components"} or data.get("schema") != INVENTORY_SCHEMA or not isinstance(components, list) or any(not isinstance(item, str) for item in components):
        raise ValueError("installed component inventory has an invalid shape")
    return tuple(components)


def write_inventory(home: Path, components: tuple[str, ...]) -> None:
    _atomic_write(inventory_path(home), json.dumps({"schema": INVENTORY_SCHEMA, "components": list(components)}, sort_keys=True).encode())


def capture_inventory_snapshot(home: object) -> InventorySnapshot:
    return InventorySnapshot(_read_regular(inventory_path(Path(home))))


def restore_inventory_snapshot(home: object, snapshot: InventorySnapshot) -> None:
    target = inventory_path(Path(home))
    if snapshot.contents is None:
        _unlink_regular(target)
    else:
        _atomic_write(target, snapshot.contents)


def read_journal(home: Path) -> Journal | None:
    contents = _read_regular(_journal_path(home))
    if contents is None:
        return None
    return Journal.decode(json.loads(contents, object_pairs_hook=_unique_object))


def write_journal(home: Path, journal: Journal) -> None:
    _atomic_write(_journal_path(home), json.dumps(journal.encode(), sort_keys=True).encode())


def clear_journal(home: Path) -> None:
    _unlink_regular(_journal_path(home))


@contextmanager
def transaction_lock(home: Path) -> Iterator[None]:
    target = inventory_path(home).parent / "lifecycle.lock"
    _ensure_directory(target.parent)
    if os.path.lexists(target):
        _read_regular(target)
    descriptor = os.open(target, os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0), 0o600)
    acquired = False
    try:
        _lock(descriptor)
        acquired = True
        yield
    finally:
        if acquired:
            _unlock(descriptor)
        os.close(descriptor)


def _journal_path(home: Path) -> Path:
    return inventory_path(home).parent / "inflight.json"


def _read_regular(target: Path) -> bytes | None:
    if not os.path.lexists(target.parent):
        _validate_real_path(target.parent, require_exists=False)
        return None
    _validate_real_path(target.parent, require_exists=True)
    if not os.path.lexists(target):
        return None
    metadata = target.lstat()
    if _is_link(metadata) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"transaction storage refuses non-regular path: {target}")
    descriptor = os.open(target, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
            raise ValueError(f"transaction storage path changed while reading: {target}")
        return os.read(descriptor, opened.st_size)
    finally:
        os.close(descriptor)


def _atomic_write(target: Path, contents: bytes) -> None:
    _ensure_directory(target.parent)
    if os.path.lexists(target):
        _read_regular(target)
    from uuid import uuid4
    temporary = target.parent / f".{target.name}.{uuid4().hex}.tmp"
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(contents)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, target)
        sync_parent_directory(target.parent)
    except BaseException:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise


def _unlink_regular(target: Path) -> None:
    if not os.path.lexists(target):
        return
    _read_regular(target)
    target.unlink()
    sync_parent_directory(target.parent)


def _ensure_directory(target: Path) -> None:
    absolute = _absolute(target)
    _validate_real_path(absolute, require_exists=False)
    current = Path(absolute.anchor)
    for part in absolute.parts[1:]:
        current /= part
        try:
            metadata = current.lstat()
        except FileNotFoundError:
            try:
                current.mkdir(mode=0o700)
            except FileExistsError:
                pass
            metadata = current.lstat()
        if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
            raise ValueError(f"transaction storage requires a real directory: {current}")


def _lock(descriptor: int) -> None:
    if os.name == "nt":
        import msvcrt
        try:
            msvcrt.locking(descriptor, msvcrt.LK_NBLCK, 1)
            return
        except OSError as error:
            if error.errno in {errno.EACCES, errno.EDEADLK}:
                raise PreAdmissionError("another getcodexy component operation is active") from error
            raise
    import fcntl
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as error:
        raise PreAdmissionError("another getcodexy component operation is active") from error


def _unlock(descriptor: int) -> None:
    if os.name == "nt":
        import msvcrt
        os.lseek(descriptor, 0, os.SEEK_SET)
        msvcrt.locking(descriptor, msvcrt.LK_UNLCK, 1)
        return
    import fcntl
    fcntl.flock(descriptor, fcntl.LOCK_UN)


def _is_link(metadata: os.stat_result) -> bool:
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return stat.S_ISLNK(metadata.st_mode) or bool(getattr(metadata, "st_file_attributes", 0) & reparse)


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("transaction storage has duplicate keys")
        result[key] = value
    return result
