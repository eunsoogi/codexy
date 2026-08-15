"""Final shard receipt persistence."""

import time

from profile_rust_receipts import digest, write


def finish_receipt(
    receipt_path,
    spec,
    arguments,
    workload,
    head,
    index_tree,
    run_id,
    run_attempt,
    status,
    failed,
    ignored,
    elapsed,
    observed_tests,
    expected_tests,
    listed_targets,
    phases,
    started_epoch,
    success: bool,
) -> bool:
    if receipt_path is not None:
        receipt = {
            "schema": SCHEMA,
            "state": "PASS" if success and elapsed <= 270 else "FAIL",
            "shard": spec.name,
            "platform": "windows" if arguments.windows else "posix",
            "argv": workload,
            "head": head,
            "index_tree": index_tree,
            "run_id": run_id,
            "run_attempt": run_attempt,
            "status": status,
            "failed": failed,
            "ignored": ignored,
            "elapsed": elapsed,
            "tests": sorted(observed_tests.elements()),
            "digest": digest(observed_tests),
            "listed_digest": digest(expected_tests),
            "physical_targets": sorted(listed_targets),
            "started": phases.get("profiler-started-epoch", started_epoch),
            "finished": time.time(),
            "workload_receipt": phases.get("workload-receipt-json"),
        }
        write(receipt_path, receipt)
        print(
            f"shard\t{spec.name}\t{receipt['state']}\t{sum(expected_tests.values())}\t{receipt['digest']}"
        )
        success = success and receipt["state"] == "PASS"

        return success
