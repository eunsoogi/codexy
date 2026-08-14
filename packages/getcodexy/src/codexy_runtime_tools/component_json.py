"""Strict JSON boundary for component inventory, diagnostics, and lifecycle state."""

from __future__ import annotations

import json
import math
from typing import Any, Callable

MAX_JSON_DEPTH = 128

def loads(text: str | bytes, *, object_pairs_hook: Callable[[list[tuple[str, Any]]], Any] | None = None) -> Any:
    """Parse standards JSON and reject JavaScript-only non-finite constants."""
    try:
        value = json.loads(text, object_pairs_hook=object_pairs_hook, parse_constant=_reject_constant, parse_float=_finite_float)
    except RecursionError as error:
        raise ValueError("component JSON nesting exceeds safe limit") from error
    _validate_depth(value)
    return value


def _validate_depth(value: Any) -> None:
    pending = [(value, 0)]
    while pending:
        current, depth = pending.pop()
        if depth > MAX_JSON_DEPTH:
            raise ValueError("component JSON nesting exceeds safe limit")
        if isinstance(current, dict):
            pending.extend((item, depth + 1) for item in current.values())
        elif isinstance(current, list):
            pending.extend((item, depth + 1) for item in current)


def _reject_constant(value: str) -> None:
    raise ValueError(f"component JSON rejects non-finite constant: {value}")


def _finite_float(value: str) -> float:
    parsed = float(value)
    if not math.isfinite(parsed):
        raise ValueError(f"component JSON rejects non-finite number: {value}")
    return parsed
