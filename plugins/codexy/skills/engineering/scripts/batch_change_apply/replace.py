"""Single-file replacement with source and destination rechecks."""

from __future__ import annotations

import os
import tempfile
from pathlib import Path
from threading import Event
from typing import Callable, Mapping

from .diff import unified
from .errors import ApplyError, ApplyInterrupted
from .paths import content_matches, read_regular, state_matches
from .validation import artifact, read_target, source, target


def _copy_atomic(target_path: Path, data: bytes, event: Event | None) -> Path:
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{target_path.name}.codexy-apply-", dir=target_path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as output:
            for offset in range(0, len(data), 1024 * 1024):
                if event is not None and event.is_set():
                    raise ApplyInterrupted("interrupted during output staging")
                output.write(data[offset : offset + 1024 * 1024])
            output.flush()
            os.fsync(output.fileno())
        return temporary
    except BaseException:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise


def _check_source(root: Path, item: Mapping[str, object]) -> None:
    path, expected = source(root, item)
    current = read_regular(path, "original")[1]
    expected_without_path = {
        key: value for key, value in expected.items() if key != "path"
    }
    if not state_matches(current, expected_without_path):
        raise ApplyError("original-changed")


def apply_one(
    item: Mapping[str, object],
    *,
    root: Path,
    results_root: Path,
    cancellation_event: Event | None,
    before_replace: Callable[[Path], None] | None,
) -> tuple[str, str, str, Mapping[str, object] | None]:
    target_path, output_path = target(root, item)
    _, data, expected_output = artifact(item, results_root)
    before = read_target(target_path)
    old_data = before[0] if before is not None else b""
    diff = unified(old_data, data, output_path)
    if before is not None and content_matches(before[1], expected_output):
        return "completed", "already-applied", diff, before[1]
    temporary = _copy_atomic(target_path, data, cancellation_event)
    try:
        if cancellation_event is not None and cancellation_event.is_set():
            raise ApplyInterrupted("interrupted before replacement")
        _check_source(root, item)
        current = read_target(target_path)
        if (before is None) != (current is None):
            raise ApplyError("destination-changed")
        if before is not None and current is not None and not content_matches(current[1], before[1]):
            if content_matches(current[1], expected_output):
                return "completed", "already-applied", diff, current[1]
            raise ApplyError("destination-changed")
        if before_replace is not None:
            before_replace(target_path)
        if cancellation_event is not None and cancellation_event.is_set():
            raise ApplyInterrupted("interrupted before replacement")
        os.replace(temporary, target_path)
        descriptor = os.open(
            target_path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
        )
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
        after_data, after_state = read_regular(target_path, "applied output")
        if not content_matches(after_state, expected_output) or after_data != data:
            raise ApplyError("readback-mismatch")
        return "completed", "applied", diff, after_state
    except BaseException:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise
