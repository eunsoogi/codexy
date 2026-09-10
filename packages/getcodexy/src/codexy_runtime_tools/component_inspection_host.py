"""Trusted host setup used by component inspection."""

from __future__ import annotations

import subprocess
from enum import Enum
from pathlib import Path
from typing import Callable

from .github_pre_session import trusted_codex
from .pre_session import _find_codex, _run


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]


class ProbeStage(str, Enum):
    EXECUTABLE = "codex-executable"
    PLUGIN_LIST = "codex-plugin-list"
    MARKETPLACE_LIST = "codex-marketplace-list"


def host(
    home: Path, codex: Path | None, runner: Runner | None
) -> tuple[Path, Runner, None] | tuple[None, None, ProbeStage]:
    try:
        return (
            trusted_codex(codex or _find_codex()),
            runner or (lambda command: _run(command, home)),
            None,
        )
    except (OSError, RuntimeError, ValueError):
        return None, None, ProbeStage.EXECUTABLE
