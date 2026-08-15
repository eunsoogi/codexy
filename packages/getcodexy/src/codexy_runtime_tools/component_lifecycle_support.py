"""Shared types and trusted executable admission for lifecycle operations."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Callable

from .component_transaction_identity import operation_id
from .component_transaction_state import PreAdmissionError
from .github_pre_session import trusted_codex
from .pre_session import _find_codex


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]


class HostExecutableError(PreAdmissionError):
    """The requested Codex executable was rejected before transaction admission."""


def operation_identifier(value: str | None) -> str:
    identifier = operation_id(value)
    if value is not None and value != identifier:
        raise ValueError("operation ID must be a safe op- identifier")
    return identifier


def host_executable(codex: Path | None) -> Path:
    try:
        return trusted_codex(codex or _find_codex())
    except (OSError, RuntimeError, ValueError) as error:
        raise HostExecutableError(str(error)) from error
