"""Payload and checkout helpers for core hook invocation tests."""

from __future__ import annotations

from collections.abc import Generator, Mapping
from contextlib import contextmanager
import json
import os
import subprocess
import tempfile
from pathlib import Path

TIMING_ENV = "CODEXY_CORE_HOOK_TIMING_FILE"


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
    if cwd is not None:
        value["cwd"] = str(cwd)
    return json.dumps(value).encode()


def worker_creation_input() -> dict[str, object]:
    return {
        "model": "gpt-6-luna",
        "thinking": "max",
        "target": {
            "type": "project",
            "projectId": "test-project",
            "environment": {"type": "worktree"},
        },
    }


def authenticated_thread_delivery_context(
    recipient: str, model: str, thinking: str
) -> dict[str, object]:
    """Add required host route fields to an allowed-path test fixture."""
    sender = "test-orchestrator"
    return {
        "session_id": sender,
        "codexy_thread_delivery": {
            "schema": "codexy.thread-delivery.v2",
            "authenticated": True,
            "direction": "root_to_child",
            "sender_thread_id": sender,
            "target_thread_id": recipient,
            "target_model": model,
            "target_thinking": thinking,
        },
    }


def authenticated_thread_delivery_payload(
    event: str,
    tool_input: Mapping[str, object],
    *,
    cwd: Path | None = None,
) -> bytes:
    """Encode a test-only host envelope for an allowed-delivery fixture."""
    payload = _authenticated_thread_delivery_payload_data(event, tool_input, cwd)
    return json.dumps(payload).encode()


def _authenticated_thread_delivery_payload_data(
    event: str,
    tool_input: Mapping[str, object],
    cwd: Path | None,
) -> dict[str, object]:
    """Assemble one synthetic host payload so fixture metadata stays consistent."""
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
        "tool_input": tool_input,
    }
    if cwd is not None:
        payload["cwd"] = str(cwd)
    payload.update(authenticated_thread_delivery_context(recipient, model, thinking))
    return payload


def hook_invocation_payload(
    plugin: Path,
    stem: str,
    tool: str,
    tool_input: Mapping[str, object],
) -> bytes:
    """Build a hook fixture, adding route authority only for a valid delivery."""
    event = "PreToolUse"
    cwd = plugin.parent.resolve()
    if (
        stem == "codexy-thread-delivery"
        and tool == "mcp__codex_app__send_message_to_thread"
    ):
        return authenticated_thread_delivery_payload(event, tool_input, cwd=cwd)
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


def sized_authenticated_thread_delivery_payload(size: int) -> bytes:
    """Build a valid delivery fixture at an exact size for the launcher limit test."""
    tool_input = {
        "threadId": "test-worker",
        "model": "gpt-6-luna",
        "thinking": "max",
        "padding": "",
    }
    payload = _authenticated_thread_delivery_payload_data(
        "PreToolUse", tool_input, None
    )
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
