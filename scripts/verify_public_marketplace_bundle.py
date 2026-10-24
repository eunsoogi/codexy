#!/usr/bin/env python3
"""Download and verify the exact public marketplace bundle for CI."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

from public_marketplace_bundle_support import (
    CONTRACT,
    LEGACY_UPGRADE_VERSION,
    SHA256,
    base_contract,
    canonical_version,
    contract_tag,
    fail,
    read_json,
    run,
    safe_extract,
    verify_bundle,
    verify_contents,
)


def resolve_automatic_public_baseline(repository: str, current_version: str) -> str:
    # Reuse the authenticated release list, then independently verify the chosen release below.
    resolver = Path(__file__).resolve().with_name("resolve_prior_public_version.py")
    version = run(
        sys.executable,
        str(resolver),
        repository,
        current_version,
        "--prefer-published-target",
    ).strip()
    return canonical_version(version, "authenticated automatic public baseline")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument(
        "--legacy-upgrade",
        action="store_true",
        help="verify the published package retained for legacy upgrade coverage",
    )
    args = parser.parse_args()
    root = Path.cwd()
    current_contract = read_json(root / CONTRACT)
    current_tag, _ = contract_tag(current_contract, "current release contract")
    override = os.environ.get("PRIOR_PUBLIC_VERSION", "")
    automatic_event = os.environ.get("GITHUB_EVENT_NAME") in {"pull_request", "push"}
    repository = os.environ.get("GITHUB_REPOSITORY", "").strip()
    if not repository:
        fail("GITHUB_REPOSITORY is required")
    baseline_source = "current checkout release contract"
    baseline_commit = ""
    sha = ""
    previous_tag = None
    if not args.legacy_upgrade and not override:
        sha = os.environ.get("BASE_SHA", "").strip()
        if sha and sha != "0" * 40:
            previous_contract = base_contract(root, sha)
            previous_tag, _ = contract_tag(
                previous_contract, f"baseline release contract {sha}"
            )
    # Keep the historical upgrade input independent from the selected public baseline.
    if args.legacy_upgrade:
        version = LEGACY_UPGRADE_VERSION
        release_tag = f"v{version}"
        baseline_source = "verified legacy upgrade fixture"
    # Select one stable baseline from an explicit override or the prior checkout contract.
    elif override:
        version = canonical_version(override, "PRIOR_PUBLIC_VERSION")
        release_tag = f"v{version}"
        baseline_source = "PRIOR_PUBLIC_VERSION"
    # Keep a changed base contract as the baseline. Resolve public versions only
    # when both tags match or an automatic event has no base tag.
    elif automatic_event and (previous_tag is None or previous_tag == current_tag):
        version = resolve_automatic_public_baseline(
            repository, current_tag.removeprefix("v")
        )
        release_tag = f"v{version}"
        baseline_source = "authenticated public release selection for automatic CI"
    elif previous_tag is not None and previous_tag != current_tag:
        release_tag = previous_tag
        version = previous_tag.removeprefix("v")
        baseline_source = f"release contract at BASE_SHA {sha}"
        baseline_commit = sha
    else:
        release_tag = current_tag
        version = current_tag.removeprefix("v")
    release_json = json.loads(
        run(
            "gh",
            "release",
            "view",
            release_tag,
            "--repo",
            repository,
            "--json",
            "tagName,isDraft,isPrerelease,assets",
        )
    )
    if (
        release_json.get("tagName") != release_tag
        or release_json.get("isDraft")
        or release_json.get("isPrerelease")
    ):
        fail(f"{release_tag} is not a published stable release")
    assets = [
        asset
        for asset in release_json.get("assets", [])
        if asset.get("name") == "codexy-marketplace-bundle.tar.gz"
    ]
    if (
        len(assets) != 1
        or not isinstance(assets[0].get("digest"), str)
        or not assets[0]["digest"].startswith("sha256:")
    ):
        fail(f"{release_tag} has no uniquely identified marketplace bundle asset")
    expected_digest = assets[0]["digest"][len("sha256:") :].lower()
    if not SHA256.fullmatch(expected_digest):
        fail("GitHub marketplace bundle asset digest is invalid")
    output_dir = args.output_dir
    if output_dir.exists():
        fail(f"refusing to reuse public release output directory: {output_dir}")
    download_dir = output_dir / "download"
    download_dir.mkdir(parents=True)
    run(
        "gh",
        "release",
        "download",
        release_tag,
        "--repo",
        repository,
        "--dir",
        str(download_dir),
        "--pattern",
        "codexy-marketplace-bundle.tar.gz",
        "--pattern",
        "runtime-release-receipt.json",
    )
    bundle = download_dir / "codexy-marketplace-bundle.tar.gz"
    receipt_path = download_dir / "runtime-release-receipt.json"
    if not bundle.is_file() or not receipt_path.is_file():
        fail("public release download did not contain the bundle and receipt")
    receipt = read_json(receipt_path)
    # Verify the downloaded archive against GitHub's asset digest and the signed release receipt.
    digest = verify_bundle(bundle, receipt, release_tag, expected_digest)
    extract = output_dir / "extracted"
    safe_extract(bundle, extract)
    # Validate the receipt inventory; only the historical upgrade fixture may emit a Watcher path.
    legacy_watcher = verify_contents(extract, receipt, version)
    print(
        json.dumps(
            {
                "release_tag": release_tag,
                "version": version,
                "baseline_source": baseline_source,
                "baseline_commit": baseline_commit,
                "bundle_sha256": digest,
                "extracted_root": str(extract.resolve()),
                "legacy_upgrade_fixture": version == LEGACY_UPGRADE_VERSION,
                "legacy_watcher_binary": (
                    str(legacy_watcher.resolve())
                    if legacy_watcher is not None
                    else None
                ),
            },
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
