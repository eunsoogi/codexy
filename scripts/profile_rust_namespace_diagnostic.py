#!/usr/bin/env python3
"""Measure existing suite_all namespaces on one disposable Windows runner."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

NAMESPACES = ("agent", "child_a", "child_b", "hook", "loc", "policy", "system", "workflow")
TOLERANCE = 0.05
RUNNING = re.compile(r"running (\d+) tests")


def execute(command: list[str], root: Path, log: Path) -> dict[str, object]:
    started = time.perf_counter()
    completed = subprocess.run(command, cwd=root, text=True, capture_output=True, check=False)
    elapsed = time.perf_counter() - started
    log.write_text(completed.stdout + completed.stderr, encoding="utf-8")
    running = RUNNING.search(completed.stdout)
    return {
        "command": command,
        "wall_seconds": elapsed,
        "selected_test_count": int(running.group(1)) if running else None,
        "exit_status": completed.returncode,
        "log": log.name,
    }


def source_state(root: Path) -> dict[str, object]:
    files = subprocess.run(["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True).stdout.split(b"\0")
    digest = hashlib.sha256()
    for raw in filter(None, files):
        relative = raw.decode("utf-8")
        digest.update(raw + b"\0")
        digest.update((root / relative).read_bytes())
    status = subprocess.run(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, check=True, capture_output=True, text=True).stdout.strip()
    return {"head": head, "clean": not status, "status": status.splitlines(), "source_digest": digest.hexdigest()}


def listed_tests(binary: str, root: Path) -> list[str]:
    completed = subprocess.run([binary, "--list"], cwd=root, check=True, capture_output=True, text=True)
    return [line.removesuffix(": test") for line in completed.stdout.splitlines() if line.endswith(": test")]


def binaries(root: Path, artifact_dir: Path) -> tuple[dict[str, str], dict[str, object]]:
    command = ["cargo", "test", "--locked", "--test", "suite_all", "--test", "suite_archive", "--no-run", "--message-format=json"]
    started = time.perf_counter()
    completed = subprocess.run(command, cwd=root, check=False, capture_output=True, text=True)
    elapsed = time.perf_counter() - started
    (artifact_dir / "build.log").write_text(completed.stdout + completed.stderr, encoding="utf-8")
    if completed.returncode:
        raise RuntimeError(f"build command {' '.join(command)} exited {completed.returncode}")
    resolved: dict[str, str] = {}
    for line in completed.stdout.splitlines():
        record = json.loads(line)
        target = record.get("target", {})
        if record.get("reason") == "compiler-artifact" and target.get("name") in {"suite_all", "suite_archive"}:
            if executable := record.get("executable"):
                resolved[target["name"]] = executable
    missing = {"suite_all", "suite_archive"} - resolved.keys()
    if missing:
        raise RuntimeError(f"missing test binaries: {', '.join(sorted(missing))}")
    return resolved, {"command": command, "wall_seconds": elapsed, "exit_status": completed.returncode, "log": "build.log"}


def environment() -> dict[str, str | None]:
    names = ("RUST_TEST_THREADS", "RUSTFLAGS", "CARGO_BUILD_JOBS", "CARGO_PROFILE_TEST_DEBUG")
    return {name: os.environ.get(name) for name in names}


def assess(before: dict[str, object], after: dict[str, object], all_tests: list[str], partitions: list[dict[str, object]], archive: dict[str, object], direct: dict[str, object]) -> dict[str, object]:
    partition_count = sum(int(item["selected_test_count"] or 0) for item in partitions)
    partition_seconds = sum(float(item["wall_seconds"]) for item in partitions)
    direct_seconds = float(direct["wall_seconds"])
    tolerance_seconds = direct_seconds * TOLERANCE
    statuses = [int(item["exit_status"]) for item in [*partitions, archive, direct]]
    names = [name for namespace in NAMESPACES for name in all_tests if name.startswith(f"{namespace}::")]
    validity = {
        "all_exited_zero": all(status == 0 for status in statuses),
        "source_unchanged": before == after,
        "source_clean": bool(before["clean"]) and bool(after["clean"]),
        "inventory_accounted": len(names) == len(all_tests) == partition_count and len(set(names)) == len(names),
        "partition_sum_within_tolerance": abs(partition_seconds - direct_seconds) <= tolerance_seconds,
    }
    owners = [item["namespace"] for item in partitions if float(item["wall_seconds"]) >= 60.0]
    return {
        "tolerance": TOLERANCE,
        "direct_suite_all_seconds": direct_seconds,
        "partition_sum_seconds": partition_seconds,
        "tolerance_seconds": tolerance_seconds,
        "suite_all_test_inventory": len(all_tests),
        "partition_test_inventory": partition_count,
        "validity": validity,
        "valid": all(validity.values()),
        "namespaces_at_or_above_60_seconds": owners,
        "static_owner_mapping": "not-assessed",
        "implementation_b_candidate": False,
        "decision": "static-owner-mapping-required" if owners else "no-implementation-lane",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    arguments = parser.parse_args()
    root, artifact_dir = arguments.root.resolve(), arguments.artifact_dir.resolve()
    artifact_dir.mkdir(parents=True, exist_ok=True)
    report: dict[str, object] = {"schema": "codexy.rust-namespace-diagnostic/v1", "environment": environment()}
    try:
        report["source_before"] = source_state(root)
        paths, build = binaries(root, artifact_dir)
        report["build"] = build
        all_tests = listed_tests(paths["suite_all"], root)
        partitions = []
        for index, namespace in enumerate(NAMESPACES, start=1):
            result = execute([paths["suite_all"], f"{namespace}::"], root, artifact_dir / f"{index:02d}-{namespace}.log")
            result["namespace"] = namespace
            partitions.append(result)
        archive = execute([paths["suite_archive"]], root, artifact_dir / "09-suite_archive.log")
        direct = execute([paths["suite_all"]], root, artifact_dir / "10-suite_all-direct.log")
        report.update({"partitions": partitions, "suite_archive_control": archive, "suite_all_control": direct})
        report["source_after"] = source_state(root)
        report["assessment"] = assess(report["source_before"], report["source_after"], all_tests, partitions, archive, direct)
    except (OSError, RuntimeError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        report["error"] = str(error)
    (artifact_dir / "namespace-diagnostic.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    return 0 if report.get("assessment", {}).get("valid") else 1


if __name__ == "__main__":
    raise SystemExit(main())
