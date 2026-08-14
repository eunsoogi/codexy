"""Bounded TOML admission for untrusted component diagnostic documents."""

from __future__ import annotations

import math
import tomllib
from typing import Any


MAX_TOML_BYTES = 65_536
MAX_TOML_DEPTH = 128
MAX_TOML_COLLECTION_ITEMS = 1_024
MAX_TOML_STRING_BYTES = 8_192


class ComponentTomlError(ValueError):
    """TOML cannot be safely used for component diagnostics."""


def loads(contents: bytes | str) -> object:
    """Parse a bounded TOML document or expose one typed input failure."""
    text = _text(contents)
    try:
        value = tomllib.loads(text)
    except (tomllib.TOMLDecodeError, ValueError, RecursionError, MemoryError, OverflowError) as error:
        raise ComponentTomlError("component TOML is malformed or exceeds a safe resource limit") from error
    _validate(value)
    return value


def _text(contents: bytes | str) -> str:
    try:
        if isinstance(contents, bytes):
            if len(contents) > MAX_TOML_BYTES:
                raise ComponentTomlError("component TOML document exceeds safe size")
            return contents.decode("utf-8")
        if len(contents.encode("utf-8")) > MAX_TOML_BYTES:
            raise ComponentTomlError("component TOML document exceeds safe size")
        return contents
    except UnicodeError as error:
        raise ComponentTomlError("component TOML is not valid UTF-8") from error


def _validate(value: object) -> None:
    pending: list[tuple[object, int]] = [(value, 0)]
    while pending:
        current, depth = pending.pop()
        if isinstance(current, dict):
            if depth > MAX_TOML_DEPTH or len(current) > MAX_TOML_COLLECTION_ITEMS:
                raise ComponentTomlError("component TOML exceeds safe nesting or collection limits")
            for key, item in current.items():
                _string(key)
                pending.append((item, depth + 1))
        elif isinstance(current, list):
            if depth > MAX_TOML_DEPTH or len(current) > MAX_TOML_COLLECTION_ITEMS:
                raise ComponentTomlError("component TOML exceeds safe nesting or collection limits")
            pending.extend((item, depth + 1) for item in current)
        elif isinstance(current, str):
            _string(current)
        elif isinstance(current, float) and not math.isfinite(current):
            raise ComponentTomlError("component TOML rejects non-finite numbers")


def _string(value: str) -> None:
    try:
        oversized = len(value.encode("utf-8")) > MAX_TOML_STRING_BYTES
    except UnicodeError as error:
        raise ComponentTomlError("component TOML contains an invalid string") from error
    if oversized:
        raise ComponentTomlError("component TOML string exceeds safe size")
