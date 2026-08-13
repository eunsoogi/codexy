"""Strict JSON boundary for component inventory, diagnostics, and lifecycle state."""

from __future__ import annotations

import json
import math
from typing import Any, Callable


def loads(text: str | bytes, *, object_pairs_hook: Callable[[list[tuple[str, Any]]], Any] | None = None) -> Any:
    """Parse standards JSON and reject JavaScript-only non-finite constants."""
    return json.loads(text, object_pairs_hook=object_pairs_hook, parse_constant=_reject_constant, parse_float=_finite_float)


def _reject_constant(value: str) -> None:
    raise ValueError(f"component JSON rejects non-finite constant: {value}")


def _finite_float(value: str) -> float:
    parsed = float(value)
    if not math.isfinite(parsed):
        raise ValueError(f"component JSON rejects non-finite number: {value}")
    return parsed
