"""Time utilities used by Rust profiling modules."""

from __future__ import annotations

import time


def elapsed(started: float) -> float:
    return round(max(0.0, time.perf_counter() - started), 6)
