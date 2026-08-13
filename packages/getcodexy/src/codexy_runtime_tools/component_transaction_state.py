"""No-follow durable state for component lifecycle transactions."""

from __future__ import annotations

import base64
import errno
import json
import os
import stat
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Iterator

from .component_transaction_durability import sync_parent_directory
from .component_transaction_identity import operation_id
from .updater import _absolute, _validate_real_path


INVENTORY_SCHEMA = "getcodexy.installed-component-inventory.v1"
JOURNAL_SCHEMA = "getcodexy.component-transaction.v1"


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


def write_receipt(home: Path, receipt: dict[str, object]) -> None:
    identifier = operation_id(receipt.get("operation_id") if isinstance(receipt.get("operation_id"), str) else None)
    target = inventory_path(home).parent / "receipts" / f"{identifier}.json"
    contents = json.dumps(receipt, sort_keys=True).encode()
    if os.path.lexists(target):
        if _read_regular(target) != contents:
            raise ValueError(f"operation receipt already exists: {identifier}")
        return
    _atomic_write(target, contents)


@dataclass(frozen=True)
class InventorySnapshot:
    contents: bytes | None

    @classmethod
    def capture(cls, home: Path) -> "InventorySnapshot":
        return cls(_read_regular(inventory_path(home)))

    def restore(self, home: Path) -> None:
        target = inventory_path(home)
        if self.contents is None:
            _unlink_regular(target)
        else:
            _atomic_write(target, self.contents)


@dataclass(frozen=True)
class Journal:
    identifier: str
    command: str
    requested: tuple[str, ...]
    resolved: tuple[str, ...]
    before: tuple[str, ...]
    target: tuple[str, ...]
    snapshot: InventorySnapshot
    phase: str

    def with_phase(self, phase: str) -> "Journal":
        return Journal(self.identifier, self.command, self.requested, self.resolved, self.before, self.target, self.snapshot, phase)


def read_journal(home: Path) -> Journal | None:
    contents = _read_regular(_journal_path(home))
    if contents is None:
        return None
    data = json.loads(contents, object_pairs_hook=_unique_object)
    required = {"schema", "operation_id", "command", "requested", "resolved", "before", "target", "inventory", "phase"}
    if not isinstance(data, dict) or set(data) != required or data.get("schema") != JOURNAL_SCHEMA or data.get("phase") not in {"started", "rolling-back", "committed"}:
        raise ValueError("component transaction journal has an invalid shape")
    fields = [data.get(name) for name in ("requested", "resolved", "before", "target")]
    if not all(isinstance(value, list) and all(isinstance(item, str) for item in value) for value in fields):
        raise ValueError("component transaction journal has invalid components")
    encoded = data.get("inventory")
    if not isinstance(encoded, str) or not isinstance(data.get("command"), str) or not isinstance(data.get("operation_id"), str) or operation_id(data["operation_id"]) != data["operation_id"]:
        raise ValueError("component transaction journal has invalid identifiers")
    try:
        snapshot = InventorySnapshot(base64.b64decode(encoded.encode(), validate=True) or None)
    except ValueError as error:
        raise ValueError("component transaction journal has invalid inventory") from error
    return Journal(data["operation_id"], data["command"], *(tuple(value) for value in fields), snapshot, data["phase"])


def write_journal(home: Path, journal: Journal) -> None:
    data = {"schema": JOURNAL_SCHEMA, "operation_id": journal.identifier, "command": journal.command, "requested": list(journal.requested), "resolved": list(journal.resolved), "before": list(journal.before), "target": list(journal.target), "inventory": base64.b64encode(journal.snapshot.contents or b"").decode(), "phase": journal.phase}
    _atomic_write(_journal_path(home), json.dumps(data, sort_keys=True).encode())


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
                raise RuntimeError("another getcodexy component operation is active") from error
            raise
    import fcntl
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as error:
        raise RuntimeError("another getcodexy component operation is active") from error


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
