"""Fail-closed context validation for the parallel Rust shard workflow."""
from __future__ import annotations

from collections.abc import Callable

SHARDS = "[support, agent, child, orchestration, governance, system, archive]"
JOBS = {"rust-test", "windows-rust-test", "rust-test-aggregate"}
CHECKOUT = {
    "uses": "actions/checkout@v7",
    "with": {
        "ref": "${{ github.event.pull_request.head.sha }}",
        "fetch-depth": "0",
        "persist-credentials": "false",
    },
}
WINDOWS_SETUP = ({"shell": "pwsh", "run": "scripts/install-windows-test-prerequisites.ps1"}, {"shell": "pwsh", "run": "rustup toolchain install; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; cargo fetch --locked; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }"})


def producer(job: dict[str, object], runner: str, timeout: str, command: str, receipt: str) -> bool:
    strategy = job.get("strategy")
    platform = "Windows" if runner.startswith("windows") else "Ubuntu"
    if set(job) != {"name", "runs-on", "timeout-minutes", "strategy", "steps"} or job.get("name") != f"Rust shard ({platform}, ${{{{ matrix.shard }}}})" or not isinstance(strategy, dict) or strategy != {"fail-fast": "false", "max-parallel": "7", "matrix": {"shard": SHARDS}}:
        return False
    steps = job.get("steps")
    if not isinstance(steps, list) or job.get("runs-on") != runner or job.get("timeout-minutes") != timeout or job.get("if") is not None:
        return False
    setup = WINDOWS_SETUP if runner.startswith("windows") else ({"run": "sudo apt-get update && sudo apt-get install --yes ripgrep"},)
    platform_name = "windows" if runner.startswith("windows") else "posix"
    upload = {"if": "always()", "uses": "actions/upload-artifact@v7", "with": {"name": f"rust-receipt-{platform_name}-${{{{ matrix.shard }}}}", "path": receipt, "if-no-files-found": "error"}}
    return tuple(steps) == (CHECKOUT, *setup, {"run": command}, upload)


def aggregate(job: dict[str, object]) -> bool:
    if set(job) != {"needs", "if", "runs-on", "timeout-minutes", "steps"} or job.get("needs") != "[rust-test, windows-rust-test]" or job.get("if") != "always()" or job.get("runs-on") != "ubuntu-latest" or job.get("timeout-minutes") != "6":
        return False
    steps = job.get("steps")
    if not isinstance(steps, list) or len(steps) != 3:
        return False
    checkout, download, profiler = steps
    return checkout == CHECKOUT and download == {"uses": "actions/download-artifact@v8", "with": {"pattern": "rust-receipt-*", "merge-multiple": "true", "path": "receipts"}} and profiler == {"run": "scripts/profile-rust-tests --aggregate-receipts receipts"}


def enforce_shard_workflow(jobs: dict[str, list[str]], context: Callable[[list[str]], dict[str, object]]) -> bool:
    if set(jobs) != JOBS:
        return False
    expected = (("rust-test", "ubuntu-latest", "6", "scripts/profile-rust-tests --shard ${{ matrix.shard }} --receipt receipts/posix-${{ matrix.shard }}.json", "receipts/posix-${{ matrix.shard }}.json"), ("windows-rust-test", "windows-latest", "20", "python scripts/profile-rust-tests --windows --shard ${{ matrix.shard }} --receipt receipts/windows-${{ matrix.shard }}.json", "receipts/windows-${{ matrix.shard }}.json"))
    contexts = {name: context(lines) for name, lines in jobs.items()}
    if any("cache" in "\n".join(jobs[name]).casefold() or "retry" in "\n".join(jobs[name]).casefold() or "continue-on-error" in "\n".join(jobs[name]).casefold() for name in jobs):
        return False
    return all(producer(contexts[name], runner, timeout, command, receipt) for name, runner, timeout, command, receipt in expected) and aggregate(contexts["rust-test-aggregate"])
