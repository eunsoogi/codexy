"""Observe bounded target and descendant-process profiler telemetry."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

SCRIPT_DIRECTORY = str(Path(__file__).resolve().parent)
if SCRIPT_DIRECTORY not in sys.path:
    sys.path.insert(0, SCRIPT_DIRECTORY)

from profile_rust_targets import target_name
from profile_rust_process_families import (
    family_counts,
    process_family,
    test_threads,
    valid_target,
)
from profile_rust_timing import elapsed
import signal
import threading
from typing import Callable, Iterable, Sequence


_UNOBSERVED = "not-observed"
_FAMILIES = ("git", "python", "shell", "validator", "other")
_POLL_SECONDS = 0.02


def stop_workload(process: object, job: object | None) -> None:
    if job is not None:
        job.terminate_and_wait()
        return
    if process.poll() is None:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()


class RuntimeTelemetry:
    def __init__(
        self, started: float, declared: Iterable[str], environment: dict[str, str]
    ) -> None:
        self._started = started
        self._declared = tuple(sorted(set(declared)))
        self._environment = dict(environment)
        self._events: list[tuple[str, str, float]] = []
        self._families = {name: 0 for name in _FAMILIES}
        self._error: ValueError | None = None
        self._thread: threading.Thread | None = None

    def start(
        self, capture_path: Path, process: object, snapshot: Callable[[], object]
    ) -> None:
        self._thread = threading.Thread(
            target=self._observe,
            args=(capture_path, process, snapshot),
            name="cargo-runtime-telemetry",
            daemon=True,
        )
        self._thread.start()

    def finish(self) -> str:
        if self._thread is not None:
            self._thread.join()
        if self._error is not None:
            raise self._error
        return json.dumps(
            receipt(
                self._declared,
                self._events,
                self._environment,
                [],
                family_max=self._families,
            ),
            sort_keys=True,
        )

    def _observe(
        self, capture_path: Path, process: object, snapshot: Callable[[], object]
    ) -> None:
        offset = 0
        pending = b""
        try:
            with capture_path.open("rb", buffering=0) as capture:
                while True:
                    capture.seek(offset)
                    chunk = capture.read()
                    offset += len(chunk)
                    pending += chunk
                    lines = pending.split(b"\n")
                    pending = lines.pop()
                    for line in lines:
                        self._observe_line(line.decode("utf-8", "replace"))
                    if process.poll() is not None:
                        if not chunk:
                            return
                    else:
                        self._observe_snapshot(snapshot())
                    time.sleep(_POLL_SECONDS)
        except ValueError as error:
            self._error = error

    def _observe_line(self, line: str) -> None:
        if "Running " in line:
            target = target_name(line)
            self._events.append((target, "started", elapsed(self._started)))
        elif line.lstrip().startswith("test result: "):
            started = next(
                (event for event in reversed(self._events) if event[1] == "started"),
                None,
            )
            if started is not None:
                self._events.append((started[0], "ended", elapsed(self._started)))

    def _observe_snapshot(self, value: object) -> None:
        for name, count in family_counts(process_records(value)).items():
            self._families[name] = max(self._families[name], count)


def receipt(
    declared: Sequence[str],
    events: Iterable[tuple[str, str, float]],
    environment: dict[str, str],
    windows_records: object,
    linux_records: object = _UNOBSERVED,
    family_max: dict[str, int] | None = None,
) -> dict[str, object]:
    parsed = parse_events(declared, events)
    records = process_records(windows_records)
    records += [] if linux_records == _UNOBSERVED else process_records(linux_records)
    seen: dict[int, str] = {}
    for pid, image in records:
        existing = seen.setdefault(pid, image)
        if existing != image:
            raise ValueError(f"duplicate process pid with different image: {pid}")
    families = (
        family_max if family_max is not None else family_counts(list(seen.items()))
    )
    targets = target_records(declared, parsed)
    return {
        "schema": "codexy.rust-runtime-telemetry/v1",
        "test_threads": test_threads(environment),
        "targets": targets,
        "ranked_completed_targets": sorted(
            (record for record in targets if record["state"] == "completed"),
            key=lambda record: (
                -float(record["elapsed_seconds"]),
                str(record["target"]),
            ),
        ),
        "process_families": families,
        "process_observation": "bounded-snapshot-max-family-concurrency",
    }


def parse_events(
    declared: Sequence[str], events: Iterable[tuple[str, str, float]]
) -> dict[str, dict[str, float]]:
    known = set(declared)
    parsed: dict[str, dict[str, float]] = {}
    for event in events:
        if not isinstance(event, tuple) or len(event) != 3:
            raise ValueError("malformed target record")
        target, state, moment = event
        if not isinstance(target, str) or not valid_target(target, known):
            raise ValueError(f"unknown target record: {target!r}")
        if (
            state not in {"started", "ended"}
            or not isinstance(moment, (int, float))
            or moment < 0
        ):
            raise ValueError("malformed target record")
        values = parsed.setdefault(target, {})
        if state in values or (state == "ended" and "started" not in values):
            raise ValueError(f"duplicate target record: {target}:{state}")
        values[state] = round(float(moment), 6)
    return parsed


def target_records(
    declared: Sequence[str], parsed: dict[str, dict[str, float]]
) -> list[dict[str, float | str]]:
    ordered = sorted(set(declared) | set(parsed))
    records: list[dict[str, float | str]] = []
    for target in ordered:
        values = parsed.get(target, {})
        started, ended = values.get("started"), values.get("ended")
        state = (
            "completed"
            if ended is not None
            else "started"
            if started is not None
            else "not-started"
        )
        records.append(
            {
                "target": target,
                "state": state,
                "started_seconds": started if started is not None else _UNOBSERVED,
                "ended_seconds": ended if ended is not None else _UNOBSERVED,
                "elapsed_seconds": round(ended - started, 6)
                if started is not None and ended is not None
                else _UNOBSERVED,
            }
        )
    return records


def process_records(value: object) -> list[tuple[int, str]]:
    if value is None or value == _UNOBSERVED or value == "not-applicable":
        return []
    if isinstance(value, dict):
        return sorted((int(pid), image) for pid, image in value.items())
    try:
        entries = json.loads(value) if isinstance(value, str) else value
    except json.JSONDecodeError as error:
        raise ValueError("malformed process records") from error
    if not isinstance(entries, list):
        raise ValueError("malformed process records")
    records: list[tuple[int, str]] = []
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("unknown process record")
        if set(entry) == {"pid", "error"}:
            if (
                not isinstance(entry["pid"], int)
                or entry["pid"] <= 0
                or not isinstance(entry["error"], str)
            ):
                raise ValueError("malformed process records")
            continue
        if set(entry) == {"pid", "ppid", "command"}:
            pid, image = entry["pid"], entry["command"].split(" ", 1)[0]
        elif set(entry) == {"pid", "image"}:
            pid, image = entry["pid"], entry["image"]
        else:
            raise ValueError("unknown process record")
        if (
            not isinstance(pid, int)
            or pid <= 0
            or not isinstance(image, str)
            or not image
        ):
            raise ValueError("malformed process records")
        records.append((pid, image))
    if len({pid for pid, _ in records}) != len(records):
        raise ValueError("duplicate process record")
    return records
