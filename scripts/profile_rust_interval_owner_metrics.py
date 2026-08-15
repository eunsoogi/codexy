"""Read bounded fixture-command owner intervals."""

from __future__ import annotations

from pathlib import Path
import re

from profile_rust_interval_values import aggregate

MAX_FILES = 256
MAX_BYTES = 1_048_576
MAX_RECORDS = 4096
MAX_RANKS = 256
MAX_NANOSECONDS = 300_000_000_000
FAMILIES = {"git", "python", "shell", "validator", "other"}
TARGETS = {"suite_all", "suite_archive", "other"}
SESSION = re.compile(r"^[0-9a-f]{32}$")
FILE = re.compile(r"^owner-interval-(p[1-9][0-9]*-[1-9][0-9]*)\.metrics$")


def metrics(
    path: Path | None, session: str | None, legacy_path: Path | None = None
) -> tuple[list[dict], dict]:
    expected = None
    if legacy_path is not None and legacy_path.is_dir():
        from profile_rust_interval_telemetry import read_rows

        expected = sum(
            len(intervals)
            for (_, key, _), producers in read_rows(legacy_path, session).items()
            if key == "fixture-command.output"
            for intervals in producers.values()
        )
    coverage = {
        "records": 0,
        "expected_records": expected if expected is not None else "not-observed",
        "groups": 0,
        "unattributed": expected if expected is not None else "not-observed",
        "truncated": False,
    }
    if path is None or not path.is_dir():
        return [], coverage
    ranked = []
    for (target, key, family, owner), producers in read_rows(path, session).items():
        item = aggregate(target, key, family, producers)
        item["owner"] = owner
        ranked.append(item)
    ranked.sort(
        key=lambda item: (
            -item["conservative_union_occupancy_seconds"],
            -item["count"],
            item["target"],
            item["key"],
            item["owner"],
        )
    )
    records = sum(item["count"] for item in ranked)
    coverage.update(
        {
            "records": records,
            "groups": len(ranked),
            "unattributed": max(0, expected - records) if expected is not None else 0,
            "truncated": len(ranked) > MAX_RANKS,
        }
    )
    return ranked[:MAX_RANKS], coverage


def read_rows(
    path: Path, session: str | None
) -> dict[tuple[str, str, str, str], dict[str, list[tuple[int, int]]]]:
    if session is not None and not SESSION.fullmatch(session):
        raise ValueError("invalid interval session")
    files = sorted(path.iterdir())
    if len(files) > MAX_FILES:
        raise ValueError("owner interval metric file overflow")
    rows: dict[tuple[str, str, str, str], dict[str, list[tuple[int, int]]]] = {}
    seen: set[tuple[str, int]] = set()
    count = 0
    for file in files:
        match = FILE.fullmatch(file.name)
        if not match or not file.is_file():
            raise ValueError("unknown owner interval metric file")
        if file.stat().st_size > MAX_BYTES:
            raise ValueError("owner interval metric byte overflow")
        producer = match.group(1)
        for line in file.open(encoding="utf-8"):
            count += 1
            if count > MAX_RECORDS:
                raise ValueError("owner interval metric record overflow")
            target, key, family, owner, sequence, interval = parse_row(
                line, session, producer
            )
            identity = (producer, sequence)
            if identity in seen:
                raise ValueError("duplicate owner interval sequence")
            seen.add(identity)
            rows.setdefault((target, key, family, owner), {}).setdefault(
                producer, []
            ).append(interval)
    return rows


def parse_row(
    line: str, session: str | None, producer: str
) -> tuple[str, str, str, str, int, tuple[int, int]]:
    if not line.endswith("\n"):
        raise ValueError("partial owner interval metric")
    fields = line.rstrip("\n").split("\t")
    if len(fields) != 12 or fields[:2] != ["fixture-command-owner", "v1"]:
        raise ValueError("malformed owner interval metric")
    (
        _,
        _,
        row_session,
        target,
        row_producer,
        sequence,
        key,
        family,
        source,
        number,
        start,
        end,
    ) = fields
    if (
        not SESSION.fullmatch(row_session)
        or session is not None
        and row_session != session
    ):
        raise ValueError("invalid interval session")
    if (
        row_producer != producer
        or target not in TARGETS
        or key != "fixture-command.output"
        or family not in FAMILIES
    ):
        raise ValueError("unknown owner interval identity")
    try:
        sequence, number, start, end = (
            int(value) for value in (sequence, number, start, end)
        )
    except ValueError as error:
        raise ValueError("malformed owner interval metric") from error
    if sequence < 1 or number < 1 or start < 0 or end < start or end > MAX_NANOSECONDS:
        raise ValueError("invalid owner interval bounds")
    return target, key, family, normalize_owner(source, number), sequence, (start, end)


def normalize_owner(source: str, number: int) -> str:
    parts = source.split("/")
    if any(part in {"", ".", ".."} for part in parts) or not re.fullmatch(
        r"tests/[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*", source
    ):
        raise ValueError("out-of-repository owner")
    return f"{source}:{number}"
