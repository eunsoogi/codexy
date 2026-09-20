"""Bounded local process lifecycle for the scenario core."""

from __future__ import annotations

import os
import queue
import subprocess
import threading
from dataclasses import dataclass
from time import monotonic, sleep
from typing import Callable, Mapping

from .lifecycle import terminate as _terminate


_CLEANUP_SECONDS = 1.0
_POLL_SECONDS = 0.01


@dataclass(frozen=True)
class ProcessCapture:
    reason: str
    returncode: int | None
    elapsed_seconds: float
    stdout_bytes: int
    stderr_bytes: int
    launch_error: bool = False


def _reader(
    stream,
    limit: int,
    count: list[int],
    total_count: list[int],
    count_lock: threading.Lock,
    exceeded: threading.Event,
    callback=None,
    send_input=None,
) -> None:
    buffer = bytearray()
    while True:
        chunk = stream.read1(8192)
        if not chunk:
            break
        count[0] += len(chunk)
        with count_lock:
            total_count[0] += len(chunk)
            over_limit = total_count[0] > limit
        if over_limit:
            exceeded.set()
            break
        buffer.extend(chunk)
        while b"\n" in buffer:
            line, _, remainder = buffer.partition(b"\n")
            buffer = bytearray(remainder)
            if callback is not None:
                callback(bytes(line).rstrip(b"\r"), send_input)
    if buffer and callback is not None and not exceeded.is_set():
        callback(bytes(buffer).rstrip(b"\r"), send_input)


def run_bounded(
    argv: tuple[str, ...],
    cwd,
    environment: Mapping[str, str],
    input_bytes: bytes,
    timeout_seconds: float,
    output_limit_bytes: int,
    cancellation,
    stop_event: threading.Event,
    on_stdout_line: Callable[[bytes, Callable[[bytes], None]], None],
) -> ProcessCapture:
    started = monotonic()
    options = {
        "args": list(argv),
        "cwd": str(cwd),
        "env": dict(environment),
        "stdin": subprocess.PIPE,
        "stdout": subprocess.PIPE,
        "stderr": subprocess.PIPE,
        "shell": False,
        "text": False,
    }
    if os.name == "nt":
        options["creationflags"] = getattr(
            subprocess, "CREATE_NEW_PROCESS_GROUP", 0x00000200
        ) | getattr(subprocess, "CREATE_NO_WINDOW", 0x08000000)
    else:
        options["start_new_session"] = True
        options["close_fds"] = True
    try:
        process = subprocess.Popen(**options)
    except OSError:
        return ProcessCapture(
            "launch-error", None, monotonic() - started, 0, 0, launch_error=True
        )
    process_group = None
    if os.name != "nt":
        try:
            process_group = os.getpgid(process.pid)
        except (OSError, ProcessLookupError):
            pass

    stdout_count = [0]
    stderr_count = [0]
    total_count = [0]
    count_lock = threading.Lock()
    exceeded = threading.Event()
    writer_stop = threading.Event()
    input_queue: queue.Queue[bytes | None] = queue.Queue()
    input_queue.put(input_bytes)

    def send_input(chunk: bytes) -> None:
        if not writer_stop.is_set():
            input_queue.put(chunk)

    threads = [
        threading.Thread(
            target=_reader,
            args=(
                process.stdout,
                output_limit_bytes,
                stdout_count,
                total_count,
                count_lock,
                exceeded,
                on_stdout_line,
                send_input,
            ),
            daemon=True,
        ),
        threading.Thread(
            target=_reader,
            args=(
                process.stderr,
                output_limit_bytes,
                stderr_count,
                total_count,
                count_lock,
                exceeded,
            ),
            daemon=True,
        ),
    ]
    for thread in threads:
        thread.start()

    def write_input() -> None:
        try:
            while not writer_stop.is_set():
                try:
                    chunk = input_queue.get(timeout=_POLL_SECONDS)
                except queue.Empty:
                    continue
                if chunk is None:
                    break
                process.stdin.write(chunk)
                process.stdin.flush()
        except (BrokenPipeError, OSError):
            pass
        finally:
            try:
                process.stdin.close()
            except (BrokenPipeError, OSError, ValueError):
                pass

    writer = threading.Thread(target=write_input, daemon=True)
    writer.start()
    reason = "process-exit"
    deadline = started + timeout_seconds
    while process.poll() is None:
        if stop_event.is_set():
            reason = "completed"
            _terminate(process, monotonic() + _CLEANUP_SECONDS, process_group)
            break
        if cancellation is not None and cancellation():
            reason = "cancelled"
            _terminate(process, monotonic() + _CLEANUP_SECONDS, process_group)
            break
        if exceeded.is_set():
            reason = "output-limit"
            _terminate(process, monotonic() + _CLEANUP_SECONDS, process_group)
            break
        if monotonic() >= deadline:
            reason = "timeout"
            _terminate(process, monotonic() + _CLEANUP_SECONDS, process_group)
            break
        sleep(_POLL_SECONDS)

    if reason == "process-exit":
        _terminate(process, monotonic() + _CLEANUP_SECONDS, process_group)
    writer_stop.set()
    input_queue.put(None)
    for thread in threads:
        thread.join(_CLEANUP_SECONDS)
    writer.join(_CLEANUP_SECONDS)
    if reason == "process-exit":
        if stop_event.is_set():
            reason = "completed"
        elif exceeded.is_set():
            reason = "output-limit"
    try:
        process.wait(timeout=max(0.05, _CLEANUP_SECONDS))
    except subprocess.TimeoutExpired:
        _terminate(process, monotonic() + _CLEANUP_SECONDS, process_group)
    if all(not thread.is_alive() for thread in threads):
        for stream in (process.stdin, process.stdout, process.stderr):
            try:
                stream.close()
            except (BrokenPipeError, OSError, ValueError, AttributeError):
                pass
    return ProcessCapture(
        reason,
        process.returncode,
        monotonic() - started,
        stdout_count[0],
        stderr_count[0],
    )
