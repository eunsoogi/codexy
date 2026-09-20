"""Public API for explicit, durable local batch-change resumption."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True

_DIRECTORY = Path(__file__).parent
for _path in (
    _DIRECTORY.parent / "batch_change_input",
    _DIRECTORY.parent / "batch_change_runner",
):
    if str(_path) not in sys.path:
        sys.path.insert(0, str(_path))

from constants import (  # noqa: E402
    DEFAULT_RESULTS_DIRECTORY,
    DEFAULT_STATE_DIRECTORY,
    RESUME_SCHEMA,
)
from resume_errors import ResumeError  # noqa: E402
from manifest import preview_from_path  # noqa: E402
from state import StateConflict  # noqa: E402
from workflow import resume_batch  # noqa: E402


def resume_from_path(
    workspace_root: str | Path,
    input_path: str,
    **kwargs: Any,
) -> dict[str, Any]:
    """Load a fresh #1140 preview, then resume it explicitly."""
    preview = preview_from_path(workspace_root, input_path)
    return resume_batch(preview, workspace_root=workspace_root, **kwargs)


__all__ = [
    "DEFAULT_RESULTS_DIRECTORY",
    "DEFAULT_STATE_DIRECTORY",
    "RESUME_SCHEMA",
    "ResumeError",
    "StateConflict",
    "resume_batch",
    "resume_from_path",
]
