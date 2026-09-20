"""Bounded POSIX subprocess execution for batch transformations."""

from __future__ import annotations

import os
import selectors
import signal
import subprocess
import time
from pathlib import Path
from threading import Event
from typing import Sequence


SUPPORTED_PLATFORM = os.name == "posix"
DEFAULT_OUTPUT_LIMIT_BYTES = 1024 * 1024
TERMINATION_GRACE_SECONDS = 0.25


def validate_output_limit(value: int) -> int:
    """Require a finite, positive output limit that is safe to allocate around."""
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or not 0 < value <= 64 * 1024 * 1024
    ):
        raise ValueError("output limit must be an integer between 1 and 67108864 bytes")
    return value


def _terminate_group(process: subprocess.Popen[bytes]) -> None:
    """Terminate a process group, escalating so descendants cannot survive."""
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    except PermissionError:
        try:
            process.terminate()
        except ProcessLookupError:
            return
    deadline = time.monotonic() + TERMINATION_GRACE_SECONDS
    while _group_exists(process.pid) and time.monotonic() < deadline:
        time.sleep(0.01)
    if _group_exists(process.pid):
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        except PermissionError:
            try:
                process.kill()
            except ProcessLookupError:
                pass
    try:
        process.wait(timeout=TERMINATION_GRACE_SECONDS)
    except subprocess.TimeoutExpired:
        try:
            process.kill()
        except ProcessLookupError:
            pass
        process.wait()


def _group_exists(process_group: int) -> bool:
    try:
        os.killpg(process_group, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def _exit_details(returncode: int | None) -> tuple[int | None, int | None]:
    if returncode is not None and returncode < 0:
        return None, -returncode
    return returncode, None


def run_command(
    argv: Sequence[str],
    *,
    cwd: Path,
    timeout_seconds: int | float,
    output_limit_bytes: int,
    cancellation_event: Event | None = None,
) -> dict[str, object]:
    """Run one trusted command without returning its stdout or stderr."""
    if not SUPPORTED_PLATFORM:
        raise RuntimeError("batch runner requires POSIX process-group support")
    validate_output_limit(output_limit_bytes)
    started = time.monotonic()
    try:
        process = subprocess.Popen(
            list(argv),
            cwd=cwd,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            shell=False,
            start_new_session=True,
        )
    except OSError as error:
        return {
            "state": "spawn-failed",
            "exit_code": None,
            "signal": None,
            "stdout_bytes": 0,
            "stderr_bytes": 0,
            "duration_ms": _duration_ms(started),
            "detail": error.strerror or "command could not be started",
        }

    selector = selectors.DefaultSelector()
    streams = (("stdout", process.stdout), ("stderr", process.stderr))
    counts = {"stdout": 0, "stderr": 0}
    for name, stream in streams:
        if stream is not None:
            selector.register(stream, selectors.EVENT_READ, name)
    state = "running"
    deadline = started + float(timeout_seconds)
    try:
        while process.poll() is None or selector.get_map():
            if cancellation_event is not None and cancellation_event.is_set():
                state = "cancelled"
                _terminate_group(process)
                break
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                state = "timeout"
                _terminate_group(process)
                break
            if not selector.get_map():
                time.sleep(min(remaining, 0.05))
                continue
            for key, _ in selector.select(min(remaining, 0.05)):
                try:
                    chunk = os.read(key.fileobj.fileno(), 65536)
                except OSError:
                    chunk = b""
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                counts[key.data] += len(chunk)
                if counts["stdout"] + counts["stderr"] > output_limit_bytes:
                    state = "output-limit"
                    _terminate_group(process)
                    break
            if state != "running":
                break
        if state == "running":
            process.wait()
    finally:
        if state == "running" and process.poll() is None:
            state = "timeout"
            _terminate_group(process)
        elif state == "running":
            _terminate_group(process)
        for key in list(selector.get_map().values()):
            try:
                selector.unregister(key.fileobj)
            except KeyError:
                pass
        selector.close()
        for _, stream in streams:
            if stream is not None:
                stream.close()

    exit_code, signal_number = _exit_details(process.returncode)
    if state == "running":
        state = "exited"
    return {
        "state": state,
        "exit_code": exit_code,
        "signal": signal_number,
        "stdout_bytes": counts["stdout"],
        "stderr_bytes": counts["stderr"],
        "duration_ms": _duration_ms(started),
    }


def _duration_ms(started: float) -> int:
    return max(0, int((time.monotonic() - started) * 1000))
