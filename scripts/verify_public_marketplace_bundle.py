#!/usr/bin/env python3
"""Download and verify the exact public marketplace bundle for CI."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

from public_marketplace_bundle_support import (
    CONTRACT,
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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()
    root = Path.cwd()
    current_contract = read_json(root / CONTRACT)
    current_tag, _ = contract_tag(current_contract, "current release contract")
    override = os.environ.get("PRIOR_PUBLIC_VERSION", "").strip()
    baseline_source = "current checkout release contract"
    baseline_commit = ""
    if override:
        version = canonical_version(override, "PRIOR_PUBLIC_VERSION")
        release_tag = f"v{version}"
        baseline_source = "PRIOR_PUBLIC_VERSION"
    else:
        release_tag = current_tag
        version = current_tag.removeprefix("v")
        sha = os.environ.get("BASE_SHA", "").strip()
        if sha and sha != "0" * 40:
            previous_contract = base_contract(root, sha)
            previous_tag, _ = contract_tag(
                previous_contract, f"baseline release contract {sha}"
            )
            if previous_tag != current_tag:
                release_tag = previous_tag
                version = previous_tag.removeprefix("v")
                baseline_source = f"release contract at BASE_SHA {sha}"
                baseline_commit = sha
    repository = os.environ.get("GITHUB_REPOSITORY", "").strip()
    if not repository:
        fail("GITHUB_REPOSITORY is required")
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
    digest = verify_bundle(bundle, receipt, release_tag, expected_digest)
    extract = output_dir / "extracted"
    safe_extract(bundle, extract)
    watcher = verify_contents(extract, receipt, version)
    print(
        json.dumps(
            {
                "release_tag": release_tag,
                "version": version,
                "baseline_source": baseline_source,
                "baseline_commit": baseline_commit,
                "bundle_sha256": digest,
                "watcher_binary": str(watcher.resolve()),
            },
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
