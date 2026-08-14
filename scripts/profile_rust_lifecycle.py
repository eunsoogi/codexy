"""Execute one registered Rust workload through the canonical profiler lifecycle."""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import threading
import time
from contextlib import nullcontext
from pathlib import Path

from profile_rust_accounting import declared_test_targets, linux_cargo_descendants_snapshot
from profile_rust_archive_accounting import receipt_environment, receipt_report_lines
from profile_rust_output import flush_output, observe_first_line, replay_output
from profile_rust_runtime_telemetry import RuntimeTelemetry, stop_workload
from profile_rust_telemetry import configure_metrics, telemetry
from profile_rust_windows import WindowsJob, configure_windows_test_runner, isolated_windows_test_root, launch_windows_workload


def run_workload(
    root: Path | None,
    budget_seconds: float,
    windows: bool = False,
    workload: tuple[str, ...] = ("cargo", "test", "--locked", "--all-targets"),
    declared_targets: set[str] | None = None,
) -> tuple[str, float, int, dict[str, float | str | Path]]:
    started = time.perf_counter()
    deadline = time.monotonic() + budget_seconds
    phase_names = (
        "cargo-root-status",
        "windows-job-pids-json",
        "windows-job-images-json",
        "linux-cargo-descendants-json",
    )
    phases: dict[str, float | str] = {
        "windows-job-active-zero": "not-applicable",
        **dict.fromkeys(phase_names, "not-applicable"),
    }
    phases["profiler-started-epoch"] = getattr(time, "time", time.perf_counter)()
    with tempfile.TemporaryDirectory(prefix="codexy-profile-") as directory:
        capture_path = Path(directory) / "cargo-output"
        capture_path.touch(mode=0o600)
        first_line: list[bytes] = []
        with receipt_environment(Path(directory)) as (receipt_dir, environment):
            metrics_path, command_metrics_path = configure_metrics(environment, Path(directory))
            temporary_root = isolated_windows_test_root(environment) if windows else nullcontext(None)
            with temporary_root as temp_root, capture_path.open("wb", buffering=0) as capture:
                if temp_root:
                    configure_windows_test_runner(environment, temp_root)
                job = WindowsJob() if os.name == "nt" else None
                if job is None:
                    process = subprocess.Popen(workload, cwd=root, stdout=capture, stderr=subprocess.STDOUT, start_new_session=os.name == "posix", env=environment)
                else:
                    process = launch_windows_workload(job, root, capture, workload, environment=environment)
                targets = declared_targets if declared_targets is not None else declared_test_targets(root) if root is not None and (root / "Cargo.toml").is_file() else ()
                runtime = RuntimeTelemetry(started, targets, environment)
                runtime.start(capture_path, process, lambda: job.diagnostics(process)["windows-job-images-json"] if job else json.dumps(linux_cargo_descendants_snapshot(process.pid)) if sys.platform.startswith("linux") else "not-applicable")
                observer = threading.Thread(target=observe_first_line, args=(capture_path, process, first_line), name="cargo-first-line", daemon=True)
                observer.start()
                cleanup_allowed = job is None
                try:
                    if job is None:
                        status = process.wait(timeout=budget_seconds)
                    else:
                        try:
                            status = process.wait(timeout=max(0, deadline - time.monotonic()))
                        except subprocess.TimeoutExpired:
                            phases["windows-job-active-zero"] = "deadline"
                            phases.update(job.diagnostics(process))
                            job.terminate_and_wait()
                            status = 124
                            cleanup_allowed = True
                        else:
                            phases.update(job.diagnostics(process))
                            if job.wait_for_empty_until(time.monotonic()):
                                phases["windows-job-active-zero"] = "completed"
                                cleanup_allowed = True
                            else:
                                phases["windows-job-active-zero"] = "drained"
                                job.terminate_and_wait()
                                cleanup_allowed = True
                except subprocess.TimeoutExpired:
                    if sys.platform.startswith("linux"):
                        phases["linux-cargo-descendants-json"] = json.dumps(
                            linux_cargo_descendants_snapshot(process.pid),
                            sort_keys=True,
                        )
                    stop_workload(process, job)
                    status = 124
                    cleanup_allowed = True
                except KeyboardInterrupt:
                    stop_workload(process, job); raise
                finally:
                    capture_started = time.perf_counter()
                    try: runtime_receipt = runtime.finish()
                    finally:
                        try:
                            if job is not None: job.close()
                        finally:
                            try: observer.join()
                            finally: del process
                if temp_root is not None and cleanup_allowed: temp_root.allow_cleanup()
        output = capture_path.read_bytes()
        if temp_root is not None and temp_root.cleanup != "removed": status = status or 1
        phases["archive-inspector-receipt-lines"] = receipt_report_lines(receipt_dir)
        phases["fixture-telemetry-json"] = telemetry(root, environment, metrics_path, temp_root.telemetry() if temp_root else None)
        phases["workload-receipt-json"] = runtime_receipt
        phases["windows-telemetry-json"] = phases["fixture-telemetry-json"]
        if temp_root is not None: phases["windows-temp-cleanup-receipt-json"] = json.dumps(temp_root.telemetry(), sort_keys=True)
    phases["workload-seconds"] = capture_started - started
    phases["capture-seconds"] = time.perf_counter() - capture_started
    live_output = first_line[0] if first_line else b""
    tail = output[len(live_output) :]
    replay_started = time.perf_counter()
    if tail: replay_output(tail)
    flush_output()
    phases["replay-seconds"] = time.perf_counter() - replay_started
    elapsed = budget_seconds if status == 124 else time.perf_counter() - started
    return output.decode("utf-8"), elapsed, status, phases
