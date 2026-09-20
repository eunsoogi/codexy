"""Durable, batch-local application state and locking."""

from __future__ import annotations

import fcntl
import json
import os
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Iterator
from uuid import uuid4

from .errors import ApplyError


STATE_SCHEMA = "codexy.batch-change-apply-state.v1"


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ApplyError(f"apply state contains duplicate field: {key}")
        result[key] = value
    return result


def read(path: Path) -> dict[str, Any] | None:
    if not os.path.lexists(path):
        return None
    if path.is_symlink() or not path.is_file():
        raise ApplyError(f"apply state must be a regular file: {path}")
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object
        )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        raise ApplyError(f"apply state is corrupt: {path}") from error
    if not isinstance(value, dict):
        raise ApplyError(f"apply state must be an object: {path}")
    return value


def write(path: Path, value: dict[str, Any]) -> None:
    temporary = path.parent / f".{path.name}.{os.getpid()}.{uuid4().hex}.tmp"
    payload = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    )
    try:
        with temporary.open("x", encoding="utf-8") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
        descriptor = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
    except OSError as error:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise ApplyError(
            f"cannot persist apply state: {error.strerror or error}"
        ) from error


@contextmanager
def lock(state_root: Path, batch_id: str) -> Iterator[None]:
    path = state_root / f".{batch_id}.lock"
    try:
        descriptor = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except (BlockingIOError, OSError) as error:
        if isinstance(error, BlockingIOError) or getattr(error, "errno", None) in {
            11,
            35,
        }:
            raise ApplyError(f"batch {batch_id} is already being applied") from error
        raise ApplyError(f"cannot lock apply state: {error}") from error
    try:
        yield
    finally:
        fcntl.flock(descriptor, fcntl.LOCK_UN)
        os.close(descriptor)


def new_state(
    batch_id: str,
    workspace: str,
    result_identity: str,
    item_ids: list[str],
) -> dict[str, Any]:
    return {
        "schema": STATE_SCHEMA,
        "batch_id": batch_id,
        "workspace": workspace,
        "result_identity": result_identity,
        "items": {
            item_id: {"status": "pending", "attempts": 0, "reason": None}
            for item_id in item_ids
        },
    }


def validate(
    value: dict[str, Any], batch_id: str, workspace: str, identity: str
) -> dict[str, Any]:
    required = {"schema", "batch_id", "workspace", "result_identity", "items"}
    if set(value) != required or value.get("schema") != STATE_SCHEMA:
        raise ApplyError("apply state has an invalid shape")
    if (
        value.get("batch_id") != batch_id
        or value.get("workspace") != workspace
        or value.get("result_identity") != identity
    ):
        raise ApplyError("apply state belongs to a different result or workspace")
    items = value.get("items")
    if not isinstance(items, dict):
        raise ApplyError("apply state items must be an object")
    allowed = {"pending", "in-progress", "completed", "conflict", "incomplete"}
    for item_id, entry in items.items():
        if not isinstance(item_id, str) or not isinstance(entry, dict):
            raise ApplyError("apply state contains an invalid item")
        if entry.get("status") not in allowed or not isinstance(
            entry.get("attempts"), int
        ):
            raise ApplyError(f"apply state item {item_id} is invalid")
        if entry["attempts"] < 0:
            raise ApplyError(f"apply state item {item_id} has invalid attempts")
    return value
