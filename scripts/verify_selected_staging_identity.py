#!/usr/bin/env python3
"""Reject staging metadata that is not bound to the selected activation and bytes."""

import hashlib
import json
import os
import re
import sys
import tarfile
from pathlib import Path


def reject(message: str) -> None:
    raise SystemExit(f"selected staging identity mismatch: {message}")


def read_json(path: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        reject(f"expected an object in {path}")
    return value


def verify(root: Path, receipt_path: Path, run_path: Path, archive_path: Path) -> None:
    """Cross-check every receipt field copied into the downstream release receipt."""
    expected_source = os.environ["STAGING_SOURCE_COMMIT"]
    expected_run_id = int(os.environ["STAGING_RUN_ID"])
    target_version = os.environ["TARGET_VERSION"]
    if not re.fullmatch(r"[0-9a-f]{40}", expected_source) or expected_run_id < 1:
        reject("workflow source or staging run input is invalid")

    activation = read_json(root / ".agents/plugins/runtime-activation.json")
    receipt = read_json(receipt_path)
    run = read_json(run_path)
    staged_provenance = read_json(
        receipt_path.with_name("runtime-staging-provenance.json")
    )
    selected_candidate = activation.get("candidate", {})
    staged_candidate = receipt.get("candidate", {})
    selected_source = selected_candidate.get("source", {})
    staged_source = staged_candidate.get("source", {})

    if (
        selected_source.get("commit") != expected_source
        or staged_source.get("commit") != expected_source
    ):
        reject("source commit differs from the selected activation")
    selected_tree = selected_source.get("tree")
    if not isinstance(selected_tree, str) or not re.fullmatch(
        r"[0-9a-f]{40}", selected_tree
    ):
        reject("selected source tree is invalid")
    if staged_source.get("tree") != selected_tree:
        reject("source tree differs from the selected activation")

    selected_classes = selected_candidate.get("classes", {})
    staged_classes = staged_candidate.get("classes", {})
    selected_handoff = selected_classes.get("coreHandoff")
    staged_handoff = staged_classes.get("coreHandoff")
    selected_manifest = (selected_handoff or {}).get("manifest", {})
    staged_manifest = (staged_handoff or {}).get("manifest", {})
    if (
        not selected_handoff
        or staged_manifest.get("path") != "handoff-runtime.json"
        or staged_manifest.get("sha256") != selected_manifest.get("sha256")
    ):
        reject("core handoff manifest differs from the selected activation")

    selected_has_watcher = "coreWatcherMcp" in selected_classes
    staged_has_watcher = "coreWatcherMcp" in staged_classes
    if selected_has_watcher != staged_has_watcher:
        reject("optional Watcher class presence differs from the selected activation")
    if selected_has_watcher:
        platforms = staged_classes["coreWatcherMcp"].get("platforms", {})
        selected_platforms = selected_classes["coreWatcherMcp"].get("platforms", {})
        if len(platforms) != 3 or platforms.get("linux-x86_64", {}).get(
            "sha256"
        ) != selected_platforms.get("linux-x86_64", {}).get("sha256"):
            reject("optional Watcher runtime differs from the selected activation")
    elif (
        staged_handoff != selected_handoff
        or staged_classes.get("devtoolsMcp") is None
        or staged_classes.get("devtoolsMcp") != selected_classes.get("devtoolsMcp")
    ):
        reject("runtime classes differ from the selected activation")

    run_attempt = run.get("run_attempt")
    if type(run.get("id")) is not int or run["id"] != expected_run_id:
        reject("authenticated staging run id differs from the workflow input")
    if type(run_attempt) is not int or run_attempt < 1:
        reject("authenticated staging run attempt is invalid")
    selected_artifact = selected_candidate.get("artifact", {})
    staged_artifact = staged_candidate.get("artifact", {})
    if (
        selected_artifact.get("stagingRunId") != expected_run_id
        or selected_artifact.get("stagingRunAttempt") != run_attempt
        or staged_artifact.get("stagingRunId") != expected_run_id
        or staged_artifact.get("stagingRunAttempt") != run_attempt
    ):
        reject("receipt run identity differs from the selected authenticated run")

    provenance = receipt.get("provenance", {})
    if (
        provenance != staged_provenance
        or provenance.get("runId") != expected_run_id
        or provenance.get("runAttempt") != run_attempt
    ):
        reject("receipt provenance differs from the downloaded run provenance")

    selected_digest = activation.get("artifact", {}).get("sha256")
    receipt_digest = receipt.get("artifact", {}).get("sha256")
    actual_digest = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    if receipt_digest != selected_digest or receipt_digest != actual_digest:
        reject("staged archive digest differs from the selected activation")

    try:
        with tarfile.open(archive_path, "r:gz") as archive:
            manifest = json.loads(
                archive.extractfile(
                    "plugins/codexy-devtools/.codex-plugin/plugin.json"
                ).read()
            )
    except (
        OSError,
        KeyError,
        AttributeError,
        tarfile.TarError,
        json.JSONDecodeError,
    ) as error:
        reject(f"staged marketplace manifest cannot be read: {error}")
    if manifest.get("version") != target_version:
        reject("staged marketplace version differs from the requested candidate")
    if (root / "plugins/codexy-devtools/runtime-candidate.json").exists():
        reject("stale runtime candidate metadata remains in the activation checkout")


def main() -> None:
    if len(sys.argv) != 4:
        reject("usage: verify_selected_staging_identity.py RECEIPT RUN ARCHIVE")
    verify(Path.cwd(), Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]))


if __name__ == "__main__":
    main()
