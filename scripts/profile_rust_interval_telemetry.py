"""Bounded conservative interval aggregation for Rust profiler command metrics."""

from __future__ import annotations

from pathlib import Path
import re
import secrets


MAX_FILES = 256
MAX_BYTES = 1_048_576
MAX_RECORDS = 4096
MAX_RANKS = 16
MAX_INTERVAL_NANOSECONDS = 300_000_000_000
FAMILIES = {"git", "python", "shell", "validator", "other"}
TARGETS = {"suite_all", "suite_archive", "other"}
SESSION = re.compile(r"^[0-9a-f]{32}$")
FILE = re.compile(r"^interval-(p[1-9][0-9]*-[1-9][0-9]*)\.metrics$")


def configure(environment: dict[str, str], directory: Path) -> Path:
    path = directory / "command-intervals"
    environment["CODEXY_PROFILE_INTERVAL_METRICS_DIR"] = str(path)
    environment["CODEXY_PROFILE_INTERVAL_SESSION"] = secrets.token_hex(16)
    return path


def metrics(path: Path | None, session: str | None) -> tuple[list[dict], list[dict], dict]:
    coverage = {
        "covered_boundaries": [
            "fixture-command-terminal", "wrapper-timeout-output", "wrapper-spawn",
            "wrapper-child-wait", "mcp-response", "mcp-final-wait",
        ],
        "uncovered_raw_command": ["package-cache-non-wrapper", "runtime-helper"],
    }
    if path is None or not path.is_dir():
        return [], [], coverage
    rows = read_rows(path, session)
    ranked = [aggregate(target, key, family, producers) for (target, key, family), producers in rows.items()]
    ranked.sort(key=lambda item: (-item["conservative_union_occupancy_seconds"], -item["count"], item["target"], item["key"]))
    families: dict[tuple[str, str], dict[str, list[tuple[int, int]]]] = {}
    for (target, _, family), producers in rows.items():
        current = families.setdefault((target, family), {})
        for producer, intervals in producers.items():
            current.setdefault(producer, []).extend(intervals)
    family_ranked = [aggregate(target, family, family, producers) for (target, family), producers in families.items()]
    family_ranked.sort(key=lambda item: (-item["conservative_union_occupancy_seconds"], item["target"], item["family"]))
    return ranked[:MAX_RANKS], family_ranked[:MAX_RANKS], coverage


def read_rows(path: Path, session: str | None) -> dict[tuple[str, str, str], dict[str, list[tuple[int, int]]]]:
    if session is not None and not SESSION.fullmatch(session):
        raise ValueError("invalid interval session")
    files = sorted(path.iterdir())
    if len(files) > MAX_FILES:
        raise ValueError("interval metric file overflow")
    rows: dict[tuple[str, str, str], dict[str, list[tuple[int, int]]]] = {}
    seen: set[tuple[str, int]] = set()
    count = 0
    for file in files:
        match = FILE.fullmatch(file.name)
        if not match or not file.is_file():
            raise ValueError("unknown interval metric file")
        if file.stat().st_size > MAX_BYTES:
            raise ValueError("interval metric byte overflow")
        producer = match.group(1)
        for line in file.open(encoding="utf-8"):
            count += 1
            if count > MAX_RECORDS:
                raise ValueError("interval metric record overflow")
            target, key, family, sequence, interval = parse_row(line, session, producer)
            identity = (producer, sequence)
            if identity in seen:
                raise ValueError("duplicate interval sequence")
            seen.add(identity)
            rows.setdefault((target, key, family), {}).setdefault(producer, []).append(interval)
    return rows


def parse_row(line: str, session: str | None, producer: str) -> tuple[str, str, str, int, tuple[int, int]]:
    if not line.endswith("\n"):
        raise ValueError("partial interval metric")
    fields = line.rstrip("\n").split("\t")
    if len(fields) != 10 or fields[:2] != ["command-interval", "v2"]:
        raise ValueError("malformed interval metric")
    _, _, row_session, target, row_producer, sequence, key, family, start, end = fields
    if not SESSION.fullmatch(row_session) or session is not None and row_session != session:
        raise ValueError("invalid interval session")
    if row_producer != producer or target not in TARGETS or not valid_key(key, family):
        raise ValueError("unknown interval identity")
    try:
        sequence, start, end = (int(value) for value in (sequence, start, end))
    except ValueError as error:
        raise ValueError("malformed interval metric") from error
    if sequence < 1 or start < 0 or end < start or end > MAX_INTERVAL_NANOSECONDS:
        raise ValueError("invalid interval bounds")
    return target, key, family, sequence, (start, end)


def valid_key(key: str, family: str) -> bool:
    if family not in FAMILIES:
        return False
    fixture = re.fullmatch(r"fixture-command\.(output|status|spawn)", key)
    wrapper = re.fullmatch(r"wrapper\.(output|spawn|child-wait)\.(git|python|shell|validator|other)", key)
    mcp = re.fullmatch(r"mcp-client\.(response|final-wait)", key)
    return bool(fixture or wrapper and wrapper.group(2) == family or mcp and family == "other")


def aggregate(target: str, key: str, family: str, producers: dict[str, list[tuple[int, int]]]) -> dict:
    cumulative = sum(end - start for intervals in producers.values() for start, end in intervals)
    unions = [union(intervals) for intervals in producers.values()]
    conservative = max(unions, default=0)
    return {
        "target": target,
        "key": key,
        "family": family,
        "count": sum(len(intervals) for intervals in producers.values()),
        "producer_count": len(producers),
        "cumulative_wait_seconds": round(cumulative / 1_000_000_000, 6),
        "conservative_union_occupancy_seconds": round(conservative / 1_000_000_000, 6),
        "overlap_ratio_upper_bound": round(1 - conservative / cumulative, 6) if cumulative else 0.0,
    }


def union(intervals: list[tuple[int, int]]) -> int:
    total = 0
    end = None
    for start, current_end in sorted(intervals):
        if end is None or start > end:
            total += current_end - start
            end = current_end
        elif current_end > end:
            total += current_end - end
            end = current_end
    return total
