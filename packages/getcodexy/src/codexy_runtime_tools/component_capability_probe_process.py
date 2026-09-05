"""Bounded process execution helpers for live capability probes."""

import json
import subprocess
from collections import namedtuple
from time import perf_counter


_RUN_OPTIONS = {"capture_output": True, "text": True, "timeout": 5}
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
