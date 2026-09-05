"""Bounded process execution helpers for live capability probes."""

import json
import os
import subprocess
from collections import namedtuple
from pathlib import Path
from time import perf_counter


_RUN_OPTIONS = {"capture_output": True, "text": True, "timeout": 5}
# Reserve part of the single probe deadline for bounded Windows tree cleanup.
_WINDOWS_CLEANUP_RESERVE = 1.0
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
    )
    try:
        remaining = max(0.0, deadline - perf_counter() - _WINDOWS_CLEANUP_RESERVE)
        if not remaining:
            raise subprocess.TimeoutExpired(argv, _RUN_OPTIONS["timeout"])
        stdout, stderr = process.communicate(input_text, timeout=remaining)
        return subprocess.CompletedProcess(argv, process.returncode, stdout, stderr)
    except subprocess.TimeoutExpired:
        _terminate_process_tree(process, deadline)
        raise
    finally:
        _close_process_pipes(process)
        _wait_for_exit(process, deadline)


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


def _close_process_pipes(process):
    for stream in (process.stdin, process.stdout, process.stderr):
        if stream is None:
            continue
        try:
            stream.close()
        except OSError:
            pass


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
