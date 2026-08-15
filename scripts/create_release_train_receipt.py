#!/usr/bin/env python3
"""Create the signed-by-attestation release receipt for one complete release train."""

import hashlib
import json
import os
import sys
import tarfile
from pathlib import Path

final_archive, bundle, runtime, staging_receipt, staging_run, output = map(
    Path, sys.argv[1:7]
)
tag = os.environ["RELEASE_TAG"]
target = tag.removeprefix("v")
components = json.loads(
    Path(
        "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"
    ).read_text()
)["components"]
staging = json.loads(staging_receipt.read_text())
run = json.loads(staging_run.read_text())
if any(component["version"] != target for component in components):
    raise SystemExit("component receipt version mismatch")
with tarfile.open(bundle, "r:gz") as archive:
    records = []
    for component in components:
        name = f"{component['asset']['packageRoot']}/.codex-plugin/plugin.json"
        member = archive.extractfile(name)
        content = member.read() if member else b""
        manifest = json.loads(content)
        if (
            manifest.get("name") != component["plugin"]
            or manifest.get("version") != target
        ):
            raise SystemExit(f"bundle manifest mismatch: {component['plugin']}")
        records.append(
            {
                "id": component["id"],
                "plugin": component["plugin"],
                "version": target,
                "packageRoot": component["asset"]["packageRoot"],
                "manifestSha256": hashlib.sha256(content).hexdigest(),
            }
        )
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
receipt = {
    "schema": "codexy-runtime-release-receipt/v2",
    "source": {
        "activationCommit": os.environ["ACTIVATION_COMMIT"],
        "stagingSourceCommit": os.environ["STAGING_SOURCE_COMMIT"],
    },
    "release": {"tag": tag},
    "artifact": {
        "name": "codexy-marketplace-plugin.tar.gz",
        "sha256": digest(final_archive),
    },
    "bundleArtifact": {
        "name": "codexy-marketplace-bundle.tar.gz",
        "sha256": digest(bundle),
    },
    "components": records,
    "runtimeArtifact": {
        "name": "codexy-runtime-package.tar.gz",
        "sha256": digest(runtime),
    },
    "staging": {
        "runId": int(os.environ["STAGING_RUN_ID"]),
        "runAttempt": run["run_attempt"],
        "artifactName": f"runtime-staging-{os.environ['STAGING_RUN_ID']}-{run['run_attempt']}",
    },
    "provenance": staging["provenance"],
}
output.write_text(json.dumps(receipt, sort_keys=True, separators=(",", ":")) + "\n")
