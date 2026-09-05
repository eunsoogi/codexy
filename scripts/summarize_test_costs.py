#!/usr/bin/env python3
"""Summarize GitHub job timings and Codexy profiling metrics."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path
from typing import Any

STAGES = ("checkout", "install", "compile", "link", "test")
OS_RE = re.compile(r"\(([^/]+)/")
INSTALL_WORDS = ("install", "toolchain", "prerequisite", "apt-get", "setup-python")
TEST_WORDS = ("cargo test", "pytest", "unittest", " test", "tests")


def timestamp(value: Any) -> dt.datetime | None:
    if isinstance(value, dt.datetime):
        return value if value.tzinfo else value.replace(tzinfo=dt.timezone.utc)
    if not isinstance(value, str) or not value:
        return None
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    return parsed if parsed.tzinfo else parsed.replace(tzinfo=dt.timezone.utc)


def seconds(start: Any, finish: Any) -> float | None:
    left, right = timestamp(start), timestamp(finish)
    if left is None or right is None or right < left:
        return None
    return round((right - left).total_seconds(), 3)


def number(value: Any) -> int | float | None:
    return (
        value
        if isinstance(value, (int, float)) and not isinstance(value, bool)
        else None
    )


def stage_for(step: dict[str, Any]) -> str | None:
    explicit = step.get("stage")
    if isinstance(explicit, str) and explicit in STAGES:
        return explicit
    name = str(step.get("name", "")).lower()
    if "checkout" in name:
        return "checkout"
    if any(word in name for word in INSTALL_WORDS):
        return "install"
    if "link" in name:
        return "link"
    if "compile" in name or "cargo build" in name:
        return "compile"
    if any(word in name for word in TEST_WORDS):
        return "test"
    return None


def operating_system(job: dict[str, Any]) -> str:
    explicit = job.get("os") or job.get("runner_os")
    if isinstance(explicit, str) and explicit:
        return explicit
    match = OS_RE.search(str(job.get("name", "")))
    return match.group(1).strip() if match else "unknown"


def read_metrics(job: dict[str, Any]) -> dict[str, int | None]:
    references = job.get("metrics", {})
    if not isinstance(references, dict):
        references = {}
    profile_path = references.get("profile") or references.get("profile_metrics")
    command_dir = references.get("command_dir") or references.get("command_metrics_dir")
    fixture_files = fixture_bytes = command_count = None
    if profile_path:
        fixture_files = fixture_bytes = 0
        for line in Path(profile_path).read_text(encoding="utf-8").splitlines():
            fields = line.split("\t")
            if not fields or fields[0] != "fixture-materialization":
                continue
            if len(fields) != 5:
                raise ValueError("invalid fixture-materialization metric")
            fixture_files += int(fields[2])
            fixture_bytes += int(fields[3])
            float(fields[4])
    if command_dir:
        command_count = 0
        for path in sorted(Path(command_dir).glob("command-*.metrics")):
            for line in path.read_text(encoding="utf-8").splitlines():
                fields = line.split("\t")
                if not fields or fields[0] != "command-wait":
                    continue
                if len(fields) != 6:
                    raise ValueError("invalid command-wait metric")
                command_count += int(fields[4])
                float(fields[5])
    return {
        "fixture_files": fixture_files,
        "fixture_bytes": fixture_bytes,
        "command_count": command_count,
    }


def summarize_job(job: dict[str, Any], head: str | None) -> dict[str, Any]:
    stages = {stage: None for stage in STAGES}
    for step in job.get("steps", []):
        if not isinstance(step, dict):
            continue
        stage, elapsed = (
            stage_for(step),
            seconds(step.get("started_at"), step.get("completed_at")),
        )
        if stage is not None and elapsed is not None:
            stages[stage] = round((stages[stage] or 0) + elapsed, 3)
    metrics = read_metrics(job)
    elapsed = seconds(job.get("started_at"), job.get("completed_at"))
    return {
        "id": job.get("id"),
        "job": job.get("name", "unnamed job"),
        "os": operating_system(job),
        "head": head,
        "conclusion": job.get("conclusion"),
        "runner_seconds": elapsed,
        "stage_seconds": stages,
        **metrics,
    }


def run_metadata(run: dict[str, Any], key: str, *aliases: str) -> Any:
    for candidate in (key, *aliases):
        if candidate in run:
            return run[candidate]
    return None


def summarize_run(run: dict[str, Any]) -> dict[str, Any]:
    jobs = run.get("jobs", [])
    if isinstance(jobs, dict):
        jobs = jobs.get("jobs", [])
    if not isinstance(jobs, list):
        raise ValueError("run.jobs must be a list")
    head = run_metadata(run, "head_sha", "head")
    head = head if isinstance(head, str) else None
    summaries = [summarize_job(job, head) for job in jobs if isinstance(job, dict)]
    starts = [timestamp(job.get("started_at")) for job in jobs if isinstance(job, dict)]
    finishes = [
        timestamp(job.get("completed_at")) for job in jobs if isinstance(job, dict)
    ]
    starts = [value for value in starts if value is not None]
    finishes = [value for value in finishes if value is not None]
    first = min(starts) if starts else None
    last = max(finishes) if finishes else None
    durations = [job["runner_seconds"] for job in summaries]
    complete_durations = bool(durations) and all(
        value is not None for value in durations
    )
    resource = run.get("resource_metrics", {})
    if not isinstance(resource, dict):
        resource = {}
    profile_kind = run_metadata(run, "profile_kind", "instrumentation")
    if profile_kind not in {"profiled", "unprofiled"}:
        profile_kind = (
            "profiled"
            if any(
                value is not None
                for job in summaries
                for value in (job["fixture_files"], job["command_count"])
            )
            else "unprofiled"
        )
    aggregate = {
        "job_count": len(summaries),
        "expected_job_count": run.get("expected_job_count"),
        "completed_job_count": sum(job["conclusion"] is not None for job in summaries),
        "runner_seconds": round(sum(durations), 3) if complete_durations else None,
        "critical_path_seconds": seconds(first, last),
        "first_job_started_at": first.isoformat() if first else None,
        "last_job_completed_at": last.isoformat() if last else None,
        "queue_seconds": seconds(
            run_metadata(run, "queued_at", "created_at"),
            run_metadata(run, "run_started_at"),
        ),
        "fixture_files": _sum_metric(summaries, "fixture_files"),
        "fixture_bytes": _sum_metric(summaries, "fixture_bytes"),
        "command_count": _sum_metric(summaries, "command_count"),
    }
    return {
        "run_id": run_metadata(run, "run_id", "id"),
        "run_url": run.get("run_url"),
        "workflow": run.get("workflow") or run.get("name"),
        "head": head,
        "condition": run.get("condition", "unknown"),
        "profile_kind": profile_kind,
        "observation_cost_seconds": number(run.get("observation_cost_seconds")),
        "resource_metrics": {
            name: number(resource.get(name))
            for name in ("rss_peak_bytes", "cpu_seconds", "disk_bytes")
        },
        "aggregate": aggregate,
        "jobs": summaries,
    }


def _sum_metric(jobs: list[dict[str, Any]], key: str) -> int | None:
    values = [job[key] for job in jobs]
    return (
        sum(values) if values and all(value is not None for value in values) else None
    )


def load_runs(path: str) -> list[dict[str, Any]]:
    raw = (
        json.load(sys.stdin)
        if path == "-"
        else json.loads(Path(path).read_text(encoding="utf-8"))
    )
    if isinstance(raw, dict) and isinstance(raw.get("runs"), list):
        runs = raw["runs"]
    elif isinstance(raw, dict):
        runs = [raw]
    else:
        raise ValueError("input must be a run object or an object with a runs list")
    return [run for run in runs if isinstance(run, dict)]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", help="JSON run manifest, or - for stdin")
    args = parser.parse_args(argv)
    try:
        report = {
            "schema": "codexy.ci-cost-summary.v1",
            "runs": [summarize_run(run) for run in load_runs(args.input)],
        }
    except (OSError, TypeError, ValueError, json.JSONDecodeError) as error:
        print(f"cannot summarize test costs: {error}", file=sys.stderr)
        return 2
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
