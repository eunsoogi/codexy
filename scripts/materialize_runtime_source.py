#!/usr/bin/env python3
"""Validate the immutable selected source and write its candidate projection."""

import hashlib
import json
import os
import re
from pathlib import Path

source = Path(os.environ["SOURCE_PLUGIN"])
staged = Path(os.environ["STAGED_PLUGIN"])
manifest = json.loads((source / ".codex-plugin/plugin.json").read_text())
public_release = os.environ["PUBLIC_RELEASE"] == "1"
staging_run_id = int(os.environ["STAGING_RUN_ID"])

if public_release:
    receipt = json.loads(Path(os.environ["PUBLIC_RECEIPT"]).read_text())
    provenance = receipt.get("provenance", {})
    staging = receipt.get("staging", {})
    source_identity = receipt.get("source", {})
    receipt_activation_commit = source_identity.get("activationCommit")
    if (
        receipt.get("schema")
        not in {
            "codexy-runtime-release-receipt/v1",
            "codexy-runtime-release-receipt/v2",
        }
        or receipt.get("release", {}).get("tag") != os.environ["RELEASE_TAG"]
        or source_identity.get("stagingSourceCommit")
        != os.environ["STAGING_SOURCE_COMMIT"]
        or not isinstance(receipt_activation_commit, str)
        or not re.fullmatch(r"[0-9a-f]{40}", receipt_activation_commit)
        or staging.get("runId") != staging_run_id
        or provenance.get("runId") != staging_run_id
        or provenance.get("runAttempt") != staging.get("runAttempt")
        or receipt.get("artifact", {}).get("sha256") != os.environ["STAGED_SHA"]
    ):
        raise SystemExit("public release receipt does not match selected identity")

    inventory = {}
    for path in sorted((staged / "runtime").iterdir()):
        if not path.is_file():
            raise SystemExit(
                f"public runtime inventory contains a non-file: {path.name}"
            )
        match = re.fullmatch(
            r"codexy-mcp-(lsp|codegraph)-(darwin-arm64|linux-x86_64|windows-x86_64)\.(bin|exe)",
            path.name,
        )
        if not match:
            raise SystemExit(
                f"public runtime inventory contains an unexpected file: {path.name}"
            )
        server, platform, extension = match.groups()
        expected_extension = "exe" if platform == "windows-x86_64" else "bin"
        if extension != expected_extension:
            raise SystemExit(
                f"public runtime inventory has an invalid platform: {path.name}"
            )
        platform_inventory = inventory.setdefault(platform, {})
        if server in platform_inventory:
            raise SystemExit(
                f"public runtime inventory contains a duplicate: {path.name}"
            )
        platform_inventory[server] = {
            "path": f"runtime/{path.name}",
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        }
    if not inventory or any(
        set(value) != {"lsp", "codegraph"} for value in inventory.values()
    ):
        raise SystemExit(
            "public runtime inventory must contain lsp and codegraph per platform"
        )
    candidate = {
        "source": {"commit": os.environ["STAGING_SOURCE_COMMIT"]},
        "artifact": {"stagingRunId": staging_run_id},
        "platforms": inventory,
    }
else:
    record = json.loads(Path(os.environ["ACTIVATION_RECORD"]).read_text())
    candidate = record["candidate"]
    if (
        candidate["source"]["commit"] != os.environ["STAGING_SOURCE_COMMIT"]
        or candidate["artifact"]["stagingRunId"] != staging_run_id
    ):
        raise SystemExit("activation record does not match selected identity")

if manifest.get("version") != os.environ["RELEASE_TAG"].removeprefix("v"):
    raise SystemExit("selected release does not match source plugin version")

if not public_release:
    candidate_bytes = (staged / "runtime-candidate.json").read_bytes()
    if (
        record["artifact"]["sha256"] != os.environ["STAGED_SHA"]
        or record["artifact"]["payloadManifestSha256"]
        != hashlib.sha256(candidate_bytes).hexdigest()
        or json.loads(candidate_bytes) != candidate
    ):
        raise SystemExit("private activation record does not match staged identity")

Path(os.environ["SELECTED_CANDIDATE"]).write_text(json.dumps(candidate, sort_keys=True))
legacy_dispatcher_free = os.environ["LEGACY_DISPATCHER_FREE"] == "1"
for platform, inventory in candidate["platforms"].items():
    for server, binary in inventory.items():
        path = staged / binary["path"]
        if hashlib.sha256(path.read_bytes()).hexdigest() != binary["sha256"]:
            raise SystemExit(f"selected runtime digest mismatch: {binary['path']}")
dispatcher = staged / "mcp/codexy-mcp-devtools.exe"
if not dispatcher.is_file() and not legacy_dispatcher_free:
    raise SystemExit(f"selected Windows dispatcher missing: {dispatcher}")
for server in ("lsp", "codegraph"):
    legacy = staged / "mcp" / f"codexy-mcp-{server}.exe"
    if legacy.exists() and not legacy_dispatcher_free:
        raise SystemExit(f"duplicate Windows server entrypoint remains: {legacy}")
protected = {}
for path in sorted((staged / "runtime").rglob("*")):
    if path.is_file() and not (
        legacy_dispatcher_free and path.name.endswith("-windows-x86_64.exe")
    ):
        protected[str(path.relative_to(staged))] = hashlib.sha256(
            path.read_bytes()
        ).hexdigest()
if dispatcher.is_file():
    protected[str(dispatcher.relative_to(staged))] = hashlib.sha256(
        dispatcher.read_bytes()
    ).hexdigest()
Path(os.environ["PROTECTED"]).write_text(json.dumps(protected, sort_keys=True))
