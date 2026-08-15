"""Shard receipt initialization."""

import time
from pathlib import Path

from profile_rust_receipts import SCHEMA, write
from profile_rust_reporting import (
    github_receipt_provenance,
    receipt_head,
    receipt_index_tree,
)


def begin_receipt(arguments, root: Path, spec, workload, expected_owned_targets):
    receipt_path, started_epoch, head, index_tree = (
        (arguments.receipt, time.time(), receipt_head(root), receipt_index_tree(root))
        if spec
        else (None, 0.0, "", "")
    )
    run_id, run_attempt = (
        github_receipt_provenance() if receipt_path is not None else (0, 0)
    )
    if receipt_path is not None:
        receipt_path.parent.mkdir(parents=True, exist_ok=True)
        write(
            receipt_path,
            {
                "schema": SCHEMA,
                "state": "PENDING",
                "shard": spec.name,
                "platform": "windows" if arguments.windows else "posix",
                "argv": workload,
                "head": head,
                "index_tree": index_tree,
                "run_id": run_id,
                "run_attempt": run_attempt,
                "physical_targets": sorted(expected_owned_targets),
                "started": started_epoch,
            },
        )
    return receipt_path, started_epoch, head, index_tree, run_id, run_attempt
