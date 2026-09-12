"""Shared validation for the public marketplace bundle lifecycle check."""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import tarfile
import unicodedata
from pathlib import Path

VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")
TAG = re.compile(r"v(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
CONTRACT = Path(".agents/plugins/release-publish-contract.json")
COMPONENTS = {
    "codexy": "plugins/codexy",
    "codexy-github": "plugins/codexy-github",
    "codexy-devtools": "plugins/codexy-devtools",
}


def fail(message: str) -> None:
    raise SystemExit(message)


def run(*arguments: str) -> str:
    result = subprocess.run(
        arguments,
        check=False,
        text=True,
        capture_output=True,
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        fail(f"{' '.join(arguments[:3])} failed: {detail}")
    return result.stdout


def read_json(path: Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"could not read JSON {path}: {error}")
    if not isinstance(value, dict):
        fail(f"JSON root must be an object: {path}")
    return value


def canonical_version(value: object, label: str) -> str:
    if not isinstance(value, str) or not VERSION.fullmatch(value):
        fail(f"{label} must be canonical MAJOR.MINOR.PATCH")
    if any(int(component) > 2_147_483_647 for component in value.split(".")):
        fail(f"{label} exceeds the supported version component bound")
    return value


def contract_tag(contract: dict, label: str) -> tuple[str, str]:
    bootstrap = contract.get("bootstrap")
    runtime = contract.get("runtime")
    if not isinstance(bootstrap, dict) or not isinstance(runtime, dict):
        fail(f"{label} must contain bootstrap and runtime objects")
    version = canonical_version(
        bootstrap.get("selectedVersion"), f"{label} selected version"
    )
    tag = runtime.get("selectedTag")
    if not isinstance(tag, str) or not TAG.fullmatch(tag) or tag != f"v{version}":
        fail(f"{label} selected runtime tag must be v{version}")
    return tag, version


def base_contract(root: Path, sha: str) -> dict:
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        fail("BASE_SHA must be a 40-character lowercase commit SHA")
    result = subprocess.run(
        ["git", "show", f"{sha}:{CONTRACT.as_posix()}"],
        cwd=root,
        check=False,
        text=True,
        capture_output=True,
    )
    if result.returncode:
        fail(
            f"could not read the baseline release contract at {sha}: {result.stderr.strip()}"
        )
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        fail(f"baseline release contract is invalid: {error}")
    if not isinstance(value, dict):
        fail("baseline release contract must be an object")
    return value


def safe_extract(archive_path: Path, destination: Path) -> None:
    if destination.exists():
        fail(f"refusing to reuse extraction directory: {destination}")
    destination.mkdir(parents=True)
    with tarfile.open(archive_path, "r:gz") as archive:
        members = []
        identities = set()
        total_size = 0
        for count, member in enumerate(archive, start=1):
            if count > 10_000:
                fail("public bundle contains too many entries")
            name = member.name
            normalized = name.rstrip("/")
            pieces = tuple(normalized.split("/")) if normalized else ()
            valid_name = name in {normalized, f"{normalized}/"}
            if (
                not pieces
                or not valid_name
                or any(part in {"", ".", ".."} for part in pieces)
            ):
                fail(f"unsafe public bundle entry: {name}")
            if (
                not (member.isfile() or member.isdir())
                or normalized.startswith("/")
                or "\\" in normalized
                or ":" in normalized
            ):
                fail(f"unsafe public bundle entry: {name}")
            identity = unicodedata.normalize("NFC", normalized).casefold()
            if identity in identities:
                fail(f"colliding public bundle entry: {name}")
            identities.add(identity)
            if member.isfile():
                if member.size > 52_428_800:
                    fail(f"oversized public bundle entry: {name}")
                total_size += member.size
                if total_size > 268_435_456:
                    fail("public bundle uncompressed size exceeds the configured limit")
            members.append(member)
        for member in members:
            archive.extract(member, destination, set_attrs=False)


def verify_bundle(
    bundle: Path, receipt: dict, release_tag: str, expected_digest: str
) -> str:
    release = receipt.get("release")
    receipt_tag = release.get("tag") if isinstance(release, dict) else None
    if receipt.get("schema") != "codexy-runtime-release-receipt/v2":
        fail("public release receipt schema mismatch")
    if receipt_tag != release_tag:
        fail("public release receipt tag mismatch")
    artifact = receipt.get("bundleArtifact")
    if not isinstance(artifact, dict) or artifact.get("name") != bundle.name:
        fail("public release receipt bundle identity mismatch")
    receipt_digest = artifact.get("sha256")
    if not isinstance(receipt_digest, str) or not SHA256.fullmatch(receipt_digest):
        fail("public release receipt bundle digest is invalid")
    if receipt_digest.lower() != expected_digest:
        fail("public release receipt disagrees with the GitHub asset digest")
    actual_digest = hashlib.sha256(bundle.read_bytes()).hexdigest()
    if actual_digest != receipt_digest.lower():
        fail("downloaded public bundle digest does not match its receipt")
    return actual_digest


def verify_contents(root: Path, receipt: dict, version: str) -> Path:
    records = receipt.get("components")
    if not isinstance(records, list):
        fail("public release receipt components must be an array")
    for plugin, package_root in COMPONENTS.items():
        matching = [
            record
            for record in records
            if isinstance(record, dict) and record.get("plugin") == plugin
        ]
        if len(matching) != 1:
            fail(f"public release receipt must contain exactly one {plugin} component")
        record = matching[0]
        if (
            record.get("packageRoot") != package_root
            or record.get("version") != version
        ):
            fail(f"public release receipt component identity mismatch: {plugin}")
        manifest_path = root / package_root / ".codex-plugin/plugin.json"
        manifest = read_json(manifest_path)
        if manifest.get("name") != plugin or manifest.get("version") != version:
            fail(f"public bundle manifest identity mismatch: {plugin}")
        manifest_digest = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
        if manifest_digest != record.get("manifestSha256"):
            fail(f"public bundle manifest digest mismatch: {plugin}")
    marketplace = read_json(root / ".agents/plugins/marketplace.json")
    plugins = marketplace.get("plugins")
    if not isinstance(plugins, list) or len(plugins) != len(COMPONENTS):
        fail("public bundle marketplace inventory is invalid")
    for entry in plugins:
        name = entry.get("name") if isinstance(entry, dict) else None
        source = entry.get("source") if isinstance(entry, dict) else None
        if name not in COMPONENTS or not isinstance(source, dict):
            fail("public bundle marketplace component is invalid")
        if (
            entry.get("version") != version
            or source.get("path") != f"./{COMPONENTS[name]}"
        ):
            fail("public bundle marketplace version or path is stale")
    watcher = root / "plugins/codexy/mcp/codexy-mcp-watcher.exe"
    if not watcher.is_file() or watcher.read_bytes()[:2] != b"MZ":
        fail("public bundle is missing a native Windows Watcher executable")
    return watcher
