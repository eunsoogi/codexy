"""Opaque operation identifiers for durable component transactions."""

from __future__ import annotations

import re
import uuid


_SAFE_ID = re.compile(r"op-[A-Za-z0-9_-]{1,128}\Z")


def valid_operation_id(value: object) -> bool:
    return isinstance(value, str) and _SAFE_ID.fullmatch(value) is not None


def operation_id(value: str | None) -> str:
    if value is None:
        return f"op-{uuid.uuid4().hex}"
    if valid_operation_id(value):
        return value
    raise ValueError("operation ID must be a safe op- identifier")
