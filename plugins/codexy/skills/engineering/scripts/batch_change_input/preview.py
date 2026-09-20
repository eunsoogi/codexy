"""Manifest validation and preview construction for batch changes."""

from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path
from typing import Any, Mapping

from paths import (
    InputError,
    _file_key,
    _nested_path,
    _original,
    _output,
    _root,
    _text,
)


SCHEMA = "codexy.batch-change-input.v1"
MAX_TIMEOUT_SECONDS = 24 * 60 * 60


def _mapping(value: Any, label: str) -> Mapping[str, Any]:
    if not isinstance(value, dict):
        raise InputError(f"{label} must be an object")
    return value


def _closed(value: Any, keys: set[str], label: str) -> Mapping[str, Any]:
    object_value = _mapping(value, label)
    if set(object_value) != keys:
        missing = sorted(keys - set(object_value))
        extra = sorted(set(object_value) - keys)
        details = []
        if missing:
            details.append(f"missing {', '.join(missing)}")
        if extra:
            details.append(f"unexpected {', '.join(extra)}")
        raise InputError(f"{label} fields are not closed ({'; '.join(details)})")
    return object_value


def _timeout(value: Any, label: str) -> int | float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(f"{label} must be a positive number of seconds")
    if isinstance(value, int):
        valid = 0 < value <= MAX_TIMEOUT_SECONDS
    else:
        valid = math.isfinite(value) and 0 < value <= MAX_TIMEOUT_SECONDS
    if not valid:
        raise InputError(f"{label} must be between 0 and {MAX_TIMEOUT_SECONDS} seconds")
    return value


def _argv(value: Any, label: str) -> list[str]:
    if not isinstance(value, list) or not value:
        raise InputError(f"{label} must be a non-empty argv array")
    result = []
    for index, token in enumerate(value):
        if not isinstance(token, str) or "\x00" in token:
            raise InputError(f"{label}[{index}] must be a string without NUL")
        if index == 0 and not token:
            raise InputError(f"{label}[0] must be a non-empty executable")
        result.append(token)
    return result


def _command(value: Any, label: str) -> dict[str, Any]:
    command = _closed(value, {"argv", "timeout_seconds"}, label)
    return {
        "argv": _argv(command["argv"], f"{label}.argv"),
        "timeout_seconds": _timeout(
            command["timeout_seconds"], f"{label}.timeout_seconds"
        ),
    }


def _identity(transform: dict[str, Any], validations: list[dict[str, Any]]) -> str:
    encoded = json.dumps(
        {"transform": transform, "validations": validations},
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _items(root: Path, value: Any) -> list[dict[str, Any]]:
    items = value.get("items") if isinstance(value, dict) else None
    if not isinstance(items, list) or not items:
        raise InputError("items must be a non-empty array")
    seen_ids: set[str] = set()
    seen_originals: dict[tuple[Any, ...], str] = {}
    seen_outputs: dict[tuple[Any, ...], str] = {}
    parsed: list[dict[str, Any]] = []
    for index, raw_item in enumerate(items):
        item = _closed(
            raw_item,
            {"id", "original", "output", "transform", "validations"},
            f"items[{index}]",
        )
        item_id = _text(item["id"], f"items[{index}].id")
        if item_id in seen_ids:
            raise InputError(f"duplicate item id: {item_id}")
        seen_ids.add(item_id)
        original_path, original, original_snapshot = _original(
            root, item["original"], f"items[{index}].original"
        )
        output_path, output, output_snapshot = _output(
            root, item["output"], f"items[{index}].output"
        )
        original_key = _file_key(original, original_snapshot["identity"])
        output_key = _file_key(output, output_snapshot["identity"])
        if original_key == output_key:
            raise InputError(f"item {item_id} output replaces its original")
        if original_key in seen_originals:
            raise InputError(f"duplicate original: {original}")
        if output_key in seen_outputs:
            raise InputError(f"duplicate output: {output}")
        for previous in parsed:
            if _nested_path(previous["_output_relative"], output):
                raise InputError(
                    f"outputs for items {previous['id']} and {item_id} overlap"
                )
        transform = _command(item["transform"], f"items[{index}].transform")
        validation_values = item["validations"]
        if not isinstance(validation_values, list) or not validation_values:
            raise InputError(f"items[{index}].validations must be a non-empty array")
        validations = [
            _command(command, f"items[{index}].validations[{position}]")
            for position, command in enumerate(validation_values)
        ]
        seen_originals[original_key] = item_id
        seen_outputs[output_key] = item_id
        parsed.append(
            {
                "id": item_id,
                "original": {
                    "path": original,
                    "absolute_path": str(original_path),
                    "state": original_snapshot["state"],
                },
                "output": output_snapshot["details"],
                "commands": {"transform": transform, "validations": validations},
                "command_identity": {
                    "algorithm": "sha256",
                    "value": _identity(transform, validations),
                },
                "_original_path": original_key,
                "_output_path": output_key,
                "_output_relative": output,
            }
        )
    originals = {entry["_original_path"] for entry in parsed}
    for entry in parsed:
        if entry["_output_path"] in originals:
            raise InputError(f"item {entry['id']} output depends on another original")
        entry.pop("_original_path")
        entry.pop("_output_path")
        entry.pop("_output_relative")
    return parsed


def preview(
    workspace_root: str | Path,
    document: Mapping[str, Any],
    input_source: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Return execution input without invoking commands or writing files."""
    value = _closed(document, {"items"}, "input list")
    root = _root(workspace_root)
    items = _items(root, value)
    return {
        "schema": SCHEMA,
        "status": "preview",
        "workspace": {"path": str(root)},
        "input": input_source or {"source": "in-memory"},
        "items": items,
        "item_count": len(items),
    }
