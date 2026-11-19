"""Payload and checkout helpers for core hook invocation tests."""

from __future__ import annotations

from collections.abc import Generator, Mapping
from contextlib import contextmanager
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

TIMING_ENV = "CODEXY_CORE_HOOK_TIMING_FILE"
TEST_SENDER = "test-sender"


def reconciled_delivery_store(root: Path) -> Path:
    """Prepare explicit complete rosters for unrelated delivery fixtures."""
    hooks = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"
    if str(hooks) not in sys.path:
        sys.path.insert(0, str(hooks))
    from codexy_policy.supervision_dispatch import Dispatch
    from codexy_policy.supervision_storage import open_store

    data = root / "plugin-data"
    data.mkdir(mode=0o700)
    if os.name != "nt":
        data.chmod(0o700)
    dispatch = Dispatch(open_store(str(data)))
    for identity in (
        TEST_SENDER,
        "parent",
        "known-worker",
        "another-known-thread",
        "test-worker",
    ):
        dispatch.bootstrap(identity, [])
    return data


@contextmanager
def temporary_primary_checkout() -> Generator[Path, None, None]:
    """Yield a temporary Git root that represents the primary checkout."""
    with tempfile.TemporaryDirectory(prefix="codexy-primary checkout ") as temporary:
        root = Path(temporary).resolve()
        _ = subprocess.run(["git", "init", str(root)], check=True, capture_output=True)
        yield root


def hook_payload(
    event: str,
    tool: str,
    tool_input: Mapping[str, object],
    *,
    cwd: Path | None = None,
) -> bytes:
    value = {"hook_event_name": event, "tool_name": tool, "tool_input": tool_input}
    if tool.endswith("send_message_to_thread"):
        value["session_id"] = TEST_SENDER
    if cwd is not None:
        value["cwd"] = str(cwd)
    return json.dumps(value).encode()


def worker_creation_input() -> dict[str, object]:
    return {
        "model": "gpt-6.1-terra",
        "thinking": "medium",
        "target": {
            "type": "project",
            "projectId": "test-project",
            "environment": {"type": "worktree"},
        },
    }


def thread_delivery_payload(
    event: str,
    tool_input: Mapping[str, object],
    *,
    cwd: Path | None = None,
) -> bytes:
    """Encode the ordinary request used to validate a delivery pair."""
    payload = _thread_delivery_payload_data(event, tool_input, cwd)
    return json.dumps(payload).encode()


def _thread_delivery_payload_data(
    event: str,
    tool_input: Mapping[str, object],
    cwd: Path | None,
) -> dict[str, object]:
    """Assemble a root-session request with no caller-supplied assignment."""
    recipient = tool_input.get("threadId")
    model = tool_input.get("model")
    thinking = tool_input.get("thinking")
    if not (
        isinstance(recipient, str)
        and recipient
        and isinstance(model, str)
        and model
        and isinstance(thinking, str)
        and thinking
    ):
        raise ValueError(
            "valid delivery fixtures require recipient, model, and thinking"
        )
    payload: dict[str, object] = {
        "hook_event_name": event,
        "tool_name": "mcp__codex_app__send_message_to_thread",
        "session_id": TEST_SENDER,
        "tool_input": tool_input,
    }
    if cwd is not None:
        payload["cwd"] = str(cwd)
    return payload


def hook_invocation_payload(
    plugin: Path,
    stem: str,
    tool: str,
    tool_input: Mapping[str, object],
) -> bytes:
    """Build the standard core-hook invocation envelope."""
    event = "PreToolUse"
    cwd = plugin.parent.resolve()
    if (
        stem == "codexy-thread-delivery"
        and tool == "mcp__codex_app__send_message_to_thread"
    ):
        return thread_delivery_payload(event, tool_input, cwd=cwd)
    return hook_payload(event, tool, tool_input, cwd=cwd)


def run_timed_core_hook_case(
    plugin: Path,
    stem: str,
    tool: str,
    tool_input: Mapping[str, object],
    timing_file: Path | None = None,
) -> subprocess.CompletedProcess[bytes]:
    """Run a core launcher fixture with optional timing output enabled."""
    environment = os.environ.copy()
    environment["PLUGIN_ROOT"] = str(plugin)
    if stem == "codexy-thread-delivery":
        data = plugin.parent / "plugin-data"
        if not data.exists():
            data = reconciled_delivery_store(plugin.parent)
        environment["PLUGIN_DATA"] = str(data)
    _ = environment.pop(TIMING_ENV, None)
    if timing_file is not None:
        environment[TIMING_ENV] = str(timing_file)
    path = plugin / "hooks" / f"{stem}.{'cmd' if os.name == 'nt' else 'sh'}"
    command = [str(path), "PreToolUse"]
    if os.name == "nt":
        # Run the native batch launcher through cmd.exe's documented call path.
        command = [
            os.environ.get("COMSPEC", "cmd.exe"),
            "/d",
            "/c",
            "call",
            *command,
        ]
    return subprocess.run(
        command,
        input=hook_invocation_payload(plugin, stem, tool, tool_input),
        capture_output=True,
        env=environment,
        check=False,
    )


def sized_thread_delivery_payload(size: int, event: str = "PreToolUse") -> bytes:
    """Build a valid delivery fixture at an exact size for the launcher limit test."""
    tool_input = {
        "threadId": "test-worker",
        "model": "gpt-6.1-terra",
        "thinking": "medium",
        "padding": "",
    }
    payload = _thread_delivery_payload_data(event, tool_input, None)
    padding_size = size - len(json.dumps(payload).encode())
    if padding_size < 0:
        raise ValueError(
            "requested input size is smaller than the valid delivery payload"
        )
    tool_input["padding"] = "x" * padding_size
    encoded = json.dumps(payload).encode()
    if len(encoded) != size:
        raise AssertionError(
            "sized delivery payload did not match the requested byte count"
        )
    return encoded
