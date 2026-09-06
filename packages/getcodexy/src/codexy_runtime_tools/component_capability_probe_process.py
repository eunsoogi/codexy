"""Bounded process execution helpers for live capability probes."""

import ctypes
import json
import os
import subprocess
import threading
from collections import namedtuple
from pathlib import Path
from time import perf_counter


_RUN_OPTIONS = {"capture_output": True, "text": True, "timeout": 5}
# Post-timeout OS cleanup is bounded separately from the probe's five-second wait.
_WINDOWS_CLEANUP_TIMEOUT = 1.0
_RunResult = namedtuple(
    "_RunResult",
    "returncode stdout category elapsed_seconds detail",
    defaults=(0.0, None),
)
_PROBE_DETAIL_LIMIT = 256


def _probe_diagnostics(result):
    return {
        "category": result.category,
        "returncode": result.returncode,
        "elapsed_seconds": round(result.elapsed_seconds, 6),
        "detail": result.detail,
    }


def _sanitize_detail(value):
    if value is None:
        return None
    if isinstance(value, bytes):
        value = value.decode(errors="replace")
    text = " ".join(str(value).split())
    if not text:
        return None
    return text[:_PROBE_DETAIL_LIMIT]


def _run(argv, cwd, input_text, env=None):
    started = perf_counter()
    try:
        if os.name == "nt":
            result = _run_windows(
                argv,
                cwd,
                input_text,
                env,
                started + _RUN_OPTIONS["timeout"],
            )
        else:
            result = subprocess.run(
                argv, input=input_text, cwd=cwd, env=env, **_RUN_OPTIONS
            )
    except subprocess.TimeoutExpired as error:
        return _RunResult(
            None,
            error.stdout or "",
            "timeout",
            perf_counter() - started,
            _sanitize_detail(error.stderr)
            or f"timeout after {_RUN_OPTIONS['timeout']} seconds",
        )
    except OSError:
        return _RunResult(
            None,
            "",
            "missing-launcher",
            perf_counter() - started,
            "launcher unavailable",
        )
    category = "success" if result.returncode == 0 else "nonzero-exit"
    category = "missing-launcher" if result.returncode == 127 else category
    return _RunResult(
        result.returncode,
        result.stdout,
        category,
        perf_counter() - started,
        _sanitize_detail(result.stderr),
    )


def _run_windows(argv, cwd, input_text, env, deadline):
    process = subprocess.Popen(
        argv,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd=cwd,
        env=env,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0x08000000),
    )
    cleanup_deadline = deadline
    abort_pipes = False
    try:
        remaining = max(0.0, deadline - perf_counter())
        if not remaining:
            raise subprocess.TimeoutExpired(argv, _RUN_OPTIONS["timeout"])
        stdout, stderr = process.communicate(input_text, timeout=remaining)
        return subprocess.CompletedProcess(argv, process.returncode, stdout, stderr)
    except subprocess.TimeoutExpired:
        abort_pipes = True
        cleanup_deadline = perf_counter() + _WINDOWS_CLEANUP_TIMEOUT
        _terminate_process_tree(process, cleanup_deadline)
        _cancel_windows_pipe_threads(process, cleanup_deadline)
        remaining = max(0.0, cleanup_deadline - perf_counter())
        if remaining:
            try:
                process.communicate(timeout=remaining)
            except (OSError, ValueError, subprocess.TimeoutExpired):
                pass
        raise
    finally:
        _close_process_pipes(process, abort=abort_pipes, deadline=cleanup_deadline)
        _wait_for_exit(process, cleanup_deadline)


def _terminate_process_tree(process, deadline):
    taskkill = (
        Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32" / "taskkill.exe"
    )
    result = None
    remaining = max(0.0, deadline - perf_counter())
    try:
        if remaining:
            result = subprocess.run(
                [str(taskkill), "/pid", str(process.pid), "/t", "/f"],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
                timeout=remaining,
            )
    except (OSError, subprocess.TimeoutExpired):
        pass
    if result is None or result.returncode != 0 or process.poll() is None:
        try:
            process.kill()
        except OSError:
            pass


def _close_process_pipes(process, abort=False, deadline=None):
    threads_stopped = True
    if abort:
        threads_stopped = _cancel_windows_pipe_threads(process, deadline)
    for attribute in ("stdin", "stdout", "stderr"):
        stream = getattr(process, attribute, None)
        if stream is None:
            continue
        if abort and not threads_stopped:
            _abort_process_pipe(process, attribute, stream)
            continue
        try:
            stream.close()
        except (OSError, ValueError):
            pass


def _abort_process_pipe(process, attribute, stream):
    try:
        descriptor = stream.fileno()
    except (OSError, TypeError, ValueError):
        return
    try:
        # Close the OS descriptor directly so a blocked communicate reader is
        # cancelled before any buffered stream close can wait on it.
        os.close(descriptor)
    except (OSError, TypeError, ValueError):
        pass
    finally:
        setattr(process, attribute, None)


def _cancel_windows_pipe_threads(process, deadline):
    threads = [
        getattr(process, attribute, None)
        for attribute in ("stdin_thread", "stdout_thread", "stderr_thread")
    ]
    threads = [thread for thread in threads if isinstance(thread, threading.Thread)]
    if not threads:
        return False
    for thread in threads:
        if thread.is_alive():
            _cancel_synchronous_io(thread)
    for thread in threads:
        if not thread.is_alive():
            continue
        remaining = max(0.0, deadline - perf_counter())
        if remaining:
            thread.join(remaining)
    return all(not thread.is_alive() for thread in threads)


def _cancel_synchronous_io(thread):
    native_id = thread.native_id
    if native_id is None:
        return
    try:
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        open_thread = kernel32.OpenThread
        cancel_io = kernel32.CancelSynchronousIo
        close_handle = kernel32.CloseHandle
        open_thread.argtypes = (ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong)
        open_thread.restype = ctypes.c_void_p
        cancel_io.argtypes = (ctypes.c_void_p,)
        cancel_io.restype = ctypes.c_int
        close_handle.argtypes = (ctypes.c_void_p,)
        close_handle.restype = ctypes.c_int
        handle = open_thread(0x0001, 0, native_id)
    except (AttributeError, OSError, TypeError, ctypes.ArgumentError):
        return
    if not handle:
        return
    try:
        cancel_io(handle)
    except (OSError, TypeError, ctypes.ArgumentError):
        pass
    finally:
        close_handle(handle)


def _wait_for_exit(process, deadline):
    if process.poll() is not None:
        return
    remaining = max(0.0, deadline - perf_counter())
    if not remaining:
        return
    try:
        process.wait(timeout=remaining)
    except subprocess.TimeoutExpired:
        pass


def _rpc(argv, cwd, requests):
    run = _run(argv, cwd, "\n".join(json.dumps(request) for request in requests) + "\n")
    values = {}
    for line in (run.stdout or "").splitlines():
        try:
            value = json.loads(line)
        except (ValueError, json.JSONDecodeError):
            continue
        if isinstance(value, dict) and isinstance(value.get("id"), int):
            values[value["id"]] = value
    return run, values
