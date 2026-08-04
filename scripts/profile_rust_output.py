"""Stream captured Cargo output without changing the measured workload."""

from __future__ import annotations

import sys
import time
from pathlib import Path


def replay_output(output: bytes) -> bool:
    binary = getattr(sys.stdout, "buffer", None)
    if binary is None:
        sys.stdout.write(output.decode("utf-8"))
        return False
    binary.write(output)
    return True


def flush_output() -> None:
    getattr(sys.stdout, "buffer", sys.stdout).flush()


def observe_first_line(capture_path: Path, process: object, lines: list[bytes]) -> None:
    with capture_path.open("rb", buffering=0) as capture:
        while process.poll() is None:
            capture.seek(0)
            line = capture.readline()
            if line.endswith(b"\n"):
                lines.append(line)
                replay_output(line)
                flush_output()
                return
            time.sleep(0.01)
