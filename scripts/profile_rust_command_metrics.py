"""Read bounded command-wait metrics emitted by the Rust profiler."""

from __future__ import annotations

import math
from pathlib import Path
import re

MAX_FILES = 256
MAX_BYTES = 1_048_576
MAX_RECORDS = 4096
MAX_RANKS = 16
FAMILIES = {"git", "python", "shell", "validator", "other"}


def metrics(
    metrics_path: Path | None,
) -> tuple[list[dict[str, float | int | str]], dict[str, float | int]]:
    if metrics_path is None or not metrics_path.is_dir():
        return [], {"count": 0, "cumulative_wait_seconds": 0.0}
    files = sorted(metrics_path.iterdir())
    if len(files) > MAX_FILES:
        raise ValueError("command metric file overflow")
    records = 0
    families: dict[str, str] = {}
    ranked: dict[tuple[str, str], dict[str, float | int | str]] = {}
    for path in files:
        if not path.is_file() or not re.fullmatch(
            r"command-[0-9]+\.metrics", path.name
        ):
            raise ValueError("unknown command metric file")
        if path.stat().st_size > MAX_BYTES:
            raise ValueError("command metric byte overflow")
        with path.open(encoding="utf-8") as lines:
            for raw_line in lines:
                if not raw_line.endswith("\n"):
                    raise ValueError("partial command metric")
                fields = raw_line.rstrip("\n").split("\t")
                records += 1
                if records > MAX_RECORDS:
                    raise ValueError("command metric record overflow")
                if len(fields) != 6 or fields[:2] != ["command-wait", "v1"]:
                    raise ValueError("malformed command metric")
                _, _, key, family, count_text, duration_text = fields
                if family not in FAMILIES or not valid_key(key, family):
                    raise ValueError("unknown command metric identity")
                if families.setdefault(key, family) != family:
                    raise ValueError("conflicting command metric identity")
                try:
                    count, duration = int(count_text), float(duration_text)
                except ValueError as error:
                    raise ValueError("malformed command metric") from error
                if count != 1 or not math.isfinite(duration) or duration < 0:
                    raise ValueError("malformed command metric")
                record = ranked.setdefault(
                    (key, family),
                    {
                        "key": key,
                        "family": family,
                        "count": 0,
                        "cumulative_wait_seconds": 0.0,
                    },
                )
                record["count"] += count
                record["cumulative_wait_seconds"] += duration
                if not math.isfinite(float(record["cumulative_wait_seconds"])):
                    raise ValueError("command metric duration overflow")
    attributed = [
        record
        for record in ranked.values()
        if ".unattributed:" not in str(record["key"])
    ]
    ordered = sorted(
        attributed,
        key=lambda record: (
            -float(record["cumulative_wait_seconds"]),
            -int(record["count"]),
            str(record["key"]),
        ),
    )[:MAX_RANKS]
    for record in ordered:
        record["cumulative_wait_seconds"] = round(
            float(record["cumulative_wait_seconds"]), 6
        )
    unattributed = [
        record for record in ranked.values() if ".unattributed:" in str(record["key"])
    ]
    return ordered, {
        "count": sum(int(record["count"]) for record in unattributed),
        "cumulative_wait_seconds": round(
            sum(float(record["cumulative_wait_seconds"]) for record in unattributed), 6
        ),
    }


def valid_key(key: str, family: str) -> bool:
    fixture_operations = {"output", "spawn", "status"}
    mcp_operations = {"response", "final-wait"}
    return (
        any(
            key == f"fixture-command.{operation}.unattributed:{family}"
            for operation in fixture_operations
        )
        or any(key == f"mcp-client.{operation}" for operation in mcp_operations)
        and family == "other"
    )
