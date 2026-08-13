"""Opaque operation identifiers for durable component transactions."""

from __future__ import annotations

import re
import uuid


_SAFE_ID = re.compile(r"op-[A-Za-z0-9_-]{1,128}\Z")


def operation_id(value: str | None) -> str:
    return value if value and _SAFE_ID.fullmatch(value) else f"op-{uuid.uuid4().hex}"
