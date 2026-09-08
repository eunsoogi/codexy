"""Bounded request/response probing for cancellable stdio MCP servers."""

from __future__ import annotations

import json
import subprocess
import threading
from collections import deque
from queue import Empty, Queue
from time import perf_counter

from .component_capability_probe_process import (
    _RunResult,
    _sanitize_detail,
)


class InteractiveRpc:
    """Keep one stdio server alive while dependent requests are exchanged."""

    def __init__(self, argv, cwd, timeout=5.0):
        self._started = perf_counter()
        self._timeout = timeout
        self._lines = deque()
        self._timed_out = False
        self._process = subprocess.Popen(
            argv,
            cwd=cwd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

    def request(self, request):
        if self._process.poll() is not None:
            return None
        try:
            self._process.stdin.write(json.dumps(request) + "\n")
            self._process.stdin.flush()
        except (OSError, ValueError):
            return None
        line = self._readline()
        if line is None:
            return None
        self._lines.append(line)
        try:
            value = json.loads(line)
        except (ValueError, json.JSONDecodeError):
            return None
        return value if isinstance(value, dict) else None

    def close(self):
        try:
            self._process.stdin.close()
        except (OSError, ValueError):
            pass
        try:
            self._process.wait(timeout=self._timeout)
        except subprocess.TimeoutExpired:
            self._terminate()
        stderr = self._process.stderr.read() if self._process.stderr else ""
        for stream in (self._process.stdout, self._process.stderr):
            if stream is not None:
                stream.close()
        return _RunResult(
            self._process.returncode,
            "".join(self._lines),
            self._category(),
            perf_counter() - self._started,
            _sanitize_detail(stderr),
        )

    def _readline(self):
        result = Queue(maxsize=1)

        def read():
            try:
                result.put(self._process.stdout.readline())
            except (OSError, ValueError):
                result.put("")

        threading.Thread(target=read, daemon=True).start()
        try:
            line = result.get(timeout=self._timeout)
        except Empty:
            self._timed_out = True
            self._terminate()
            return None
        return line.rstrip("\r\n") + "\n" if line else None

    def _terminate(self):
        if self._process.poll() is not None:
            return
        try:
            self._process.terminate()
            self._process.wait(timeout=1)
        except (OSError, subprocess.TimeoutExpired):
            try:
                self._process.kill()
                self._process.wait(timeout=1)
            except (OSError, subprocess.TimeoutExpired):
                pass

    def _category(self):
        if self._timed_out:
            return "timeout"
        if self._process.returncode == 127:
            return "missing-launcher"
        return "success" if self._process.returncode == 0 else "nonzero-exit"


def open_rpc(argv, cwd):
    return InteractiveRpc(argv, cwd)
