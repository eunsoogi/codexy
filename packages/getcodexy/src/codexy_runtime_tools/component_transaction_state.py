"""No-follow durable state for component lifecycle transactions."""

from __future__ import annotations

import errno
import json
import os
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

from .component_transaction_snapshot import (
    InventorySnapshot,
    _atomic_write,
    _ensure_directory,
    _read_regular,
    _unlink_regular,
)
from .component_transition_model import JOURNAL_SCHEMA, Journal
from .updater import _absolute


INVENTORY_SCHEMA = "getcodexy.installed-component-inventory.v1"
REGISTRATION_LOCK = ".codexy-agent-registration.lock"


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
    if (
        not isinstance(data, dict)
        or set(data) != {"schema", "components"}
        or data.get("schema") != INVENTORY_SCHEMA
        or not isinstance(components, list)
        or any(not isinstance(item, str) for item in components)
    ):
        raise ValueError("installed component inventory has an invalid shape")
    return tuple(components)


def write_inventory(home: Path, components: tuple[str, ...]) -> None:
    _atomic_write(
        inventory_path(home),
        json.dumps(
            {"schema": INVENTORY_SCHEMA, "components": list(components)}, sort_keys=True
        ).encode(),
    )


def capture_inventory_snapshot(home: object) -> InventorySnapshot:
    absolute_home = _absolute(str(home))
    from .component_lifecycle_finish import capture_managed_files

    return InventorySnapshot(
        _read_regular(inventory_path(absolute_home)),
        capture_managed_files(absolute_home),
    )


def restore_inventory_snapshot(home: object, snapshot: InventorySnapshot) -> None:
    absolute_home = _absolute(str(home))
    if snapshot.managed_files is not None:
        from .component_lifecycle_finish import restore_managed_files

        restore_managed_files(absolute_home, snapshot.managed_files)
    target = inventory_path(absolute_home)
    if snapshot.contents is None:
        _unlink_regular(target)
    else:
        _atomic_write(target, snapshot.contents)


def clear_stale_registration_lock(home: Path) -> None:
    """Remove only a registration lock whose recorded owner is gone.

    This is intentionally recovery-only. A fresh lifecycle operation must not
    delete a lock it cannot prove is stale, because pre-session registration
    can legitimately be active outside the lifecycle lock.
    """

    target = _absolute(home) / REGISTRATION_LOCK
    if not os.path.lexists(target):
        return
    try:
        metadata = target.lstat()
    except FileNotFoundError:
        return
    identity = (metadata.st_dev, metadata.st_ino)
    contents = _read_regular(target)
    if contents is None:
        return
    try:
        current = target.lstat()
    except FileNotFoundError:
        return
    if (current.st_dev, current.st_ino) != identity:
        raise PreAdmissionError(
            "Codexy agent registration lock changed during recovery"
        )
    try:
        value = contents.decode("ascii").strip()
        pid = int(value)
    except (UnicodeDecodeError, ValueError) as error:
        raise PreAdmissionError(
            "Codexy agent registration lock has an invalid owner"
        ) from error
    if pid <= 0:
        raise PreAdmissionError("Codexy agent registration lock has an invalid owner")
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        _unlink_regular(target, identity)
    except OSError as error:
        if error.errno == errno.ESRCH:
            _unlink_regular(target, identity)
        else:
            raise PreAdmissionError(
                "another Codexy agent registration is active or unobservable"
            ) from error
    else:
        raise PreAdmissionError("another Codexy agent registration is active")


def read_journal(home: Path) -> Journal | None:
    contents = _read_regular(_journal_path(home))
    if contents is None:
        return None
    return Journal.decode(json.loads(contents, object_pairs_hook=_unique_object))


def write_journal(home: Path, journal: Journal) -> None:
    _atomic_write(
        _journal_path(home), json.dumps(journal.encode(), sort_keys=True).encode()
    )


def clear_journal(home: Path) -> None:
    _unlink_regular(_journal_path(home))


@contextmanager
def transaction_lock(home: Path) -> Iterator[None]:
    target = inventory_path(home).parent / "lifecycle.lock"
    _ensure_directory(target.parent)
    if os.path.lexists(target):
        _read_regular(target)
    descriptor = os.open(
        target, os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0), 0o600
    )
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


def _multiline_state(line: str, state: str | None) -> tuple[str | None, int | None]:
    from .component_registration_health import _quoted_end

    escaped = lambda p: (len(line[:p]) - len(line[:p].rstrip("\\"))) % 2 == 1
    index, closed = 0, None
    while index < len(line):
        if state:
            if line.startswith(state, index) and (state == "'''" or not escaped(index)):
                state, index, closed = None, index + 3, closed or index + 3
            else:
                index += 1
            continue
        if line[index] == "#":
            break
        triple = next(
            (quote for quote in ('"""', "'''") if line.startswith(quote, index)),
            None,
        )
        if triple:
            state, index = triple, index + 3
        elif line[index] in ('"', "'"):
            index = _quoted_end(line, index) or len(line)
        else:
            index += 1
    return state, closed


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("transaction storage has duplicate keys")
        result[key] = value
    return result


def _lock(descriptor: int) -> None:
    if os.name == "nt":
        import msvcrt

        try:
            msvcrt.locking(descriptor, msvcrt.LK_NBLCK, 1)
            return
        except OSError as error:
            if error.errno in {errno.EACCES, errno.EDEADLK}:
                raise PreAdmissionError(
                    "another getcodexy component operation is active"
                ) from error
            raise
    import fcntl

    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as error:
        raise PreAdmissionError(
            "another getcodexy component operation is active"
        ) from error


def _unlock(descriptor: int) -> None:
    if os.name == "nt":
        import msvcrt

        os.lseek(descriptor, 0, os.SEEK_SET)
        msvcrt.locking(descriptor, msvcrt.LK_UNLCK, 1)
        return
    import fcntl

    fcntl.flock(descriptor, fcntl.LOCK_UN)
