"""Atomic shard receipt serialization and deterministic inventory digests."""
from __future__ import annotations

import hashlib
import json
from collections import Counter
from pathlib import Path

SCHEMA = "codexy.rust-shard.receipt/v1"


def digest(tests: Counter[str]) -> str:
    return hashlib.sha256("\n".join(sorted(tests.elements())).encode()).hexdigest()


def write(path: Path, value: object) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, sort_keys=True))
    temporary.replace(path)


def load(directory: Path) -> list[dict[str, object]]:
    receipts = []
    for path in sorted(directory.glob("*.json")):
        value = json.loads(path.read_text())
        if not isinstance(value, dict) or value.get("schema") != SCHEMA:
            raise ValueError(f"invalid shard receipt: {path}")
        receipts.append(value)
    return receipts
