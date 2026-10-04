from __future__ import annotations

from collections.abc import Iterator
from contextlib import contextmanager
import json
import subprocess
import tempfile
from pathlib import Path


@contextmanager
def temporary_primary_checkout() -> Iterator[Path]:
    with tempfile.TemporaryDirectory(prefix="codexy-primary checkout ") as temporary:
        root = Path(temporary).resolve()
        subprocess.run(["git", "init", str(root)], check=True, capture_output=True)
        yield root


def hook_payload(
    event: str,
    tool: str,
    tool_input: dict[str, object],
    *,
    cwd: Path | None = None,
) -> bytes:
    value = {"hook_event_name": event, "tool_name": tool, "tool_input": tool_input}
    if cwd is not None:
        value["cwd"] = str(cwd)
    return json.dumps(value).encode()
