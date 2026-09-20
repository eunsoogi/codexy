"""Validation and presentation helpers for selected resume results."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Mapping

from .diff import unified
from .errors import ApplyError
from .paths import (
    absolute_under,
    content_matches,
    read_optional,
    read_regular,
    relative,
    regular_parent,
)


RESUME_SCHEMA = "codexy.batch-change-resume.v1"
SUCCESS_RESOLUTIONS = {"rerun", "reuse"}


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ApplyError(f"result contains duplicate field: {key}")
        result[key] = value
    return result


def document(path_value: str) -> tuple[dict[str, Any], str]:
    try:
        raw = (
            sys.stdin.buffer.read()
            if path_value == "-"
            else Path(path_value).read_bytes()
        )
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        raise ApplyError(f"invalid resume result: {path_value}") from error
    if not isinstance(value, dict):
        raise ApplyError("resume result must be an object")
    return value, hashlib.sha256(raw).hexdigest()


def items(document_value: Mapping[str, Any], root: Path) -> list[Mapping[str, Any]]:
    if document_value.get("schema") != RESUME_SCHEMA:
        raise ApplyError("apply requires a codexy.batch-change-resume.v1 result")
    workspace_value = document_value.get("workspace")
    if not isinstance(workspace_value, Mapping) or workspace_value.get("path") != str(
        root
    ):
        raise ApplyError("resume result workspace does not match the apply workspace")
    values = document_value.get("items")
    if not isinstance(values, list) or not values:
        raise ApplyError("resume result must contain items")
    result: list[Mapping[str, Any]] = []
    seen: set[str] = set()
    for index, item in enumerate(values):
        if not isinstance(item, Mapping) or not isinstance(item.get("id"), str):
            raise ApplyError(f"resume result item {index} is invalid")
        if item["id"] in seen:
            raise ApplyError(f"resume result has duplicate item id: {item['id']}")
        seen.add(item["id"])
        result.append(item)
    return result


def successful(item: Mapping[str, Any]) -> bool:
    return (
        item.get("status") == "succeeded"
        and item.get("resolution") in SUCCESS_RESOLUTIONS
        and isinstance(item.get("original"), Mapping)
        and isinstance(item.get("result"), Mapping)
    )


def artifact(
    item: Mapping[str, Any], results_root: Path
) -> tuple[Path, bytes, Mapping[str, Any]]:
    result = item.get("result")
    output = result.get("output") if isinstance(result, Mapping) else None
    if not isinstance(output, Mapping) or output.get("validated") is not True:
        raise ApplyError("validated output is missing")
    expected = output.get("state")
    if not isinstance(expected, Mapping):
        raise ApplyError("validated output state is missing")
    path = absolute_under(results_root, output.get("path"), "validated artifact")
    data, actual = read_regular(path, "validated artifact")
    if not content_matches(actual, expected):
        raise ApplyError("validated artifact changed")
    return path, data, expected


def source(root: Path, item: Mapping[str, Any]) -> tuple[Path, Mapping[str, Any]]:
    original = item.get("original")
    if not isinstance(original, Mapping):
        raise ApplyError("original state is missing")
    path = root / relative(root, original.get("path"), "original")
    expected = original.get("state")
    if not isinstance(expected, Mapping):
        raise ApplyError("original state is missing")
    return path, expected


def target(root: Path, item: Mapping[str, Any]) -> tuple[Path, str]:
    path = relative(root, item.get("output_path"), "output")
    target_path = root / path
    regular_parent(target_path, "output")
    return target_path, path.as_posix()


def read_target(path: Path) -> tuple[bytes, Mapping[str, Any]] | None:
    return read_optional(path, "output")


def result_item(
    item: Mapping[str, Any],
    *,
    selected: bool,
    resolution: str,
    status: str,
    reason: str | None = None,
    diff: str | None = None,
    readback: Mapping[str, Any] | None = None,
) -> dict[str, Any]:
    value: dict[str, Any] = {
        "id": item["id"],
        "selected": selected,
        "resolution": resolution,
        "status": status,
        "source_resolution": item.get("resolution"),
        "source_status": item.get("status"),
        "output_path": item.get("output_path"),
        "diff": diff,
        "readback": dict(readback) if readback is not None else None,
    }
    if reason is not None:
        value["reason"] = reason
    return value


def selected_ids(items_value: list[Mapping[str, Any]], values: list[str]) -> set[str]:
    if not values or len(set(values)) != len(values):
        raise ApplyError("selection must contain one or more unique item ids")
    known = {str(item["id"]) for item in items_value}
    unknown = [item_id for item_id in values if item_id not in known]
    if unknown:
        raise ApplyError(f"selection contains unknown item: {unknown[0]}")
    return set(values)


def diff_for(
    item: Mapping[str, Any], root: Path, results_root: Path
) -> tuple[str, bytes, Mapping[str, Any], Path, str]:
    target_path, output_path = target(root, item)
    _, data, expected = artifact(item, results_root)
    before = read_target(target_path)
    old_data = before[0] if before is not None else b""
    return (
        unified(old_data, data, output_path),
        data,
        expected,
        target_path,
        output_path,
    )
