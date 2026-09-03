"""Read-only effective hook state from the trusted Codex host."""

from __future__ import annotations

import json
import os
import queue
import subprocess
import threading
import time
from pathlib import Path

from .version_lock import default_package_version


HOOK_STATE_UNAVAILABLE = "hook-state-unavailable"
HOOK_LIST_TIMEOUT_SECONDS = 5.0
_ENVIRONMENT_UNSAFE = (
    "GIT_DIR",
    "GIT_EXEC_PATH",
    "GIT_SSH",
    "GIT_SSH_COMMAND",
    "GIT_WORK_TREE",
    "SSH_ASKPASS",
    "PYTHONHOME",
    "PYTHONPATH",
)


class HookStateError(RuntimeError):
    """The trusted host did not return an inspectable hook state."""

    code = HOOK_STATE_UNAVAILABLE


def list_hooks(executable: Path, codex_home: Path) -> tuple[dict[str, object], ...]:
    """Read the trusted host registry without invoking or approving hooks."""
    try:
        cwd = Path.cwd().resolve()
    except OSError as error:
        raise HookStateError(
            "unable to determine the inspected working directory"
        ) from error
    environment = os.environ.copy()
    for name in _ENVIRONMENT_UNSAFE:
        environment.pop(name, None)
    environment.update(
        {
            "CODEX_HOME": str(codex_home),
            "GIT_CONFIG_COUNT": "0",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_TERMINAL_PROMPT": "0",
        }
    )
    try:
        process = subprocess.Popen(
            [str(executable), "app-server", "--listen", "stdio://"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            cwd=str(cwd),
            env=environment,
        )
    except OSError as error:
        raise HookStateError("trusted Codex app-server could not be started") from error

    values: queue.Queue[object] = queue.Queue()

    def read_lines() -> None:
        assert process.stdout is not None
        try:
            for line in process.stdout:
                try:
                    values.put(json.loads(line))
                except (UnicodeError, ValueError):
                    values.put(None)
        finally:
            values.put(None)

    reader = threading.Thread(target=read_lines, daemon=True)
    reader.start()
    try:
        assert process.stdin is not None
        initialize = {
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "getcodexy",
                    "version": default_package_version(),
                },
            },
        }
        process.stdin.write(json.dumps(initialize) + "\n")
        process.stdin.flush()
        response = _response(values, 1)
        if response.get("error") is not None or not isinstance(
            response.get("result"), dict
        ):
            raise HookStateError("trusted Codex app-server initialization failed")
        for request in (
            {"jsonrpc": "2.0", "method": "initialized", "params": {}},
            {
                "jsonrpc": "2.0",
                "method": "hooks/list",
                "id": 2,
                "params": {"cwds": [str(cwd)]},
            },
        ):
            process.stdin.write(json.dumps(request) + "\n")
            process.stdin.flush()
        response = _response(values, 2)
        if response.get("error") is not None:
            raise HookStateError("trusted Codex app-server rejected hooks/list")
        return _extract_hooks(response)
    except (BrokenPipeError, OSError) as error:
        raise HookStateError(
            "trusted Codex app-server stopped before hooks/list"
        ) from error
    finally:
        try:
            if process.stdin is not None:
                process.stdin.close()
        except OSError:
            pass
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=1)
        reader.join(timeout=1)
        try:
            if process.stdout is not None:
                process.stdout.close()
        except OSError:
            pass


def _extract_hooks(response: dict[str, object]) -> tuple[dict[str, object], ...]:
    result = response.get("result")
    if not isinstance(result, dict):
        raise HookStateError("hooks/list returned no result")
    return _rows(result)


def _rows(value: object) -> tuple[dict[str, object], ...]:
    if isinstance(value, dict):
        if isinstance(value.get("hooks"), list):
            value = value["hooks"]
        elif isinstance(value.get("data"), list):
            flattened = []
            for item in value["data"]:
                if not isinstance(item, dict) or not isinstance(
                    item.get("hooks"), list
                ):
                    raise HookStateError("hooks/list returned an invalid cwd entry")
                if item.get("warnings", []) or item.get("errors", []):
                    raise HookStateError("hooks/list returned warnings or errors")
                flattened.extend(item["hooks"])
            value = flattened
        else:
            raise HookStateError("hooks/list returned an invalid result")
    if not isinstance(value, (list, tuple)):
        raise HookStateError("hooks/list returned an invalid hook collection")
    rows = []
    for row in value:
        if not isinstance(row, dict) or not isinstance(row.get("key"), str):
            raise HookStateError("hooks/list returned a malformed hook entry")
        rows.append(row)
    return tuple(rows)


def _response(values: queue.Queue[object], identifier: int) -> dict[str, object]:
    deadline = time.monotonic() + HOOK_LIST_TIMEOUT_SECONDS
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise HookStateError("trusted Codex app-server hooks/list timed out")
        try:
            value = values.get(timeout=remaining)
        except queue.Empty as error:
            raise HookStateError(
                "trusted Codex app-server hooks/list timed out"
            ) from error
        if isinstance(value, dict) and value.get("id") == identifier:
            return value
