"""Acquire a separately verified, frozen official local marketplace release."""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

from .local_marketplace import _unique_object, validate_local_archive
from .package import _download
from .plugin_resolution import MarketplaceIdentity
from .public_marketplace_bundle import safe_extract, verify_bundle, verify_contents
from .updater import _validate_real_path

REPOSITORY = "eunsoogi/codexy"
BUNDLE = "codexy-marketplace-bundle.tar.gz"
RECEIPT = "runtime-release-receipt.json"
VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")
SHA256 = re.compile(r"sha256:([0-9a-f]{64})\Z")
COMMIT = re.compile(r"[0-9a-f]{40}\Z")


def _command(arguments: list[str]) -> object:
    try:
        result = subprocess.run(
            arguments,
            check=True,
            capture_output=True,
            text=True,
            timeout=120,
            env={**os.environ, "GH_HOST": "github.com"},
        )
        return json.loads(result.stdout, object_pairs_hook=_unique_object)
    except (OSError, subprocess.SubprocessError, ValueError) as error:
        # Never include CLI output, which can contain local credentials or private paths.
        raise ValueError("official release verification command failed") from error


def _asset(release: dict, name: str, tag: str) -> tuple[str, str]:
    assets = release.get("assets")
    selected = (
        [a for a in assets if isinstance(a, dict) and a.get("name") == name]
        if isinstance(assets, list)
        else []
    )
    if len(selected) != 1:
        raise ValueError("official release asset is missing or ambiguous")
    asset = selected[0]
    digest = SHA256.fullmatch(str(asset.get("digest", "")))
    url = f"https://github.com/{REPOSITORY}/releases/download/{tag}/{name}"
    if (
        digest is None
        or asset.get("browser_download_url") != url
        or asset.get("state") != "uploaded"
    ):
        raise ValueError("official release asset identity is invalid")
    return url, digest[1]


def _attestation(path: Path, activation: str) -> None:
    signer = f"{REPOSITORY}/.github/workflows/publish-version-release.yml"
    evidence = _command(
        [
            "gh",
            "attestation",
            "verify",
            str(path),
            "--repo",
            REPOSITORY,
            "--signer-workflow",
            signer,
            "--source-ref",
            "refs/heads/main",
            "--source-digest",
            activation,
            "--deny-self-hosted-runners",
            "--limit",
            "1000",
            "--format",
            "json",
        ]
    )
    if not isinstance(evidence, list) or not evidence:
        raise ValueError("official release attestation is missing")
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    # CLI policy verifies signatures and issuer; bind its verified subject to these exact bytes.
    for item in evidence:
        result = item.get("verificationResult", {}) if isinstance(item, dict) else {}
        statement = result.get("statement", {})
        subjects = statement.get("subject") if isinstance(statement, dict) else None
        if subjects != [{"name": path.name, "digest": {"sha256": digest}}]:
            raise ValueError("official release attestation subject mismatch")


def acquire_target(
    home: Path, version: str, components: tuple[str, ...]
) -> MarketplaceIdentity:
    """Leave existing roots untouched; return a new root only after independent verification."""
    if not VERSION.fullmatch(version):
        raise ValueError("target release version is not canonical")
    _validate_real_path(home, require_exists=True)
    # A fresh directory prevents stale local receipts or partial extractions from becoming trusted.
    parent = home / "getcodexy" / "marketplaces"
    _validate_real_path(parent, require_exists=False)
    parent.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix=f"v{version}-", dir=parent))
    try:
        tag = f"v{version}"
        release = _command(["gh", "api", f"repos/{REPOSITORY}/releases/tags/{tag}"])
        if (
            not isinstance(release, dict)
            or release.get("tag_name") != tag
            or release.get("draft") is not False
            or release.get("prerelease") is not False
        ):
            raise ValueError("target must be the exact official stable release")
        digests = {}
        for name in (BUNDLE, RECEIPT):
            url, digest = _asset(release, name, tag)
            path = work / name
            _download(url, path)
            if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
                raise ValueError("official release asset digest mismatch")
            digests[name] = digest
        receipt = json.loads(
            (work / RECEIPT).read_text(), object_pairs_hook=_unique_object
        )
        source = receipt.get("source") if isinstance(receipt, dict) else None
        activation = (
            source.get("activationCommit") if isinstance(source, dict) else None
        )
        if not isinstance(activation, str) or not COMMIT.fullmatch(activation):
            raise ValueError("official release activation identity is invalid")
        for name in (RECEIPT, BUNDLE):
            _attestation(work / name, activation)
        verify_bundle(work / BUNDLE, receipt, tag, digests[BUNDLE])
        root = work / "marketplace"
        safe_extract(work / BUNDLE, root)
        verify_contents(root, receipt, version)
        validate_local_archive(root, version, components)
        return MarketplaceIdentity(root, "local", str(root))
    except BaseException as error:
        shutil.rmtree(work)
        if isinstance(error, SystemExit):
            raise ValueError(str(error)) from error
        raise
