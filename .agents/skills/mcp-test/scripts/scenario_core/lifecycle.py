"""Bounded termination for a local process and its owned process group."""

from __future__ import annotations

import errno
import os
import signal
import subprocess
from contextlib import suppress
from time import monotonic, sleep

from .support import validate_platform


_POLL_SECONDS = 0.01


def _group_exists(group_id: int | None) -> bool:
    if group_id is None:
        return False
    try:
        os.killpg(group_id, 0)
    except OSError as error:
        return error.errno != errno.ESRCH
    return True


def terminate(
    process: subprocess.Popen[bytes], deadline: float, process_group: int | None = None
) -> None:
    """Terminate the process group even when its parent has already exited."""

    validate_platform()
    if process_group is None:
        with suppress(OSError, ProcessLookupError):
            process_group = os.getpgid(process.pid)
    try:
        if process_group is not None:
            os.killpg(process_group, signal.SIGTERM)
        elif process.poll() is None:
            process.terminate()
    except (OSError, ProcessLookupError):
        with suppress(OSError):
            process.terminate()
    while monotonic() < deadline and (
        process.poll() is None or _group_exists(process_group)
    ):
        sleep(_POLL_SECONDS)
    if process_group is not None:
        with suppress(OSError, ProcessLookupError):
            os.killpg(process_group, signal.SIGKILL)
    if process.poll() is None:
        with suppress(OSError):
            process.kill()
