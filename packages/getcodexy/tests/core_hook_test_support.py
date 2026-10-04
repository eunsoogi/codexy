"""Payload and checkout helpers for core hook invocation tests."""

from __future__ import annotations

from collections.abc import Generator, Mapping
from contextlib import contextmanager
import json
import subprocess
import tempfile
from pathlib import Path


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
