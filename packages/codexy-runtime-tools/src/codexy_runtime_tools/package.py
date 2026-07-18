from __future__ import annotations

import json
import hashlib
import os
import shutil
import stat
import tarfile
import urllib.request
import zipfile
from pathlib import Path
from urllib.parse import urlparse

MAX_ARCHIVE_FILES = 2_048
MAX_UNPACKED_BYTES = 512 * 1024 * 1024


def _download(url: str, destination: Path, token: str = "") -> None:
    headers = {"Accept": "application/vnd.github+json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=30) as response, destination.open("wb") as output:
        shutil.copyfileobj(response, output)


def _safe_extract_tar(archive: Path, destination: Path) -> None:
    destination_resolved = destination.resolve()
    with tarfile.open(archive, "r:gz") as package:
        members = package.getmembers()
        if len(members) > MAX_ARCHIVE_FILES:
            raise ValueError("runtime package contains too many members")
        if sum(member.size for member in members) > MAX_UNPACKED_BYTES:
            raise ValueError("runtime package exceeds the unpacked size limit")
        destinations: set[Path] = set()
        for member in members:
            if not (member.isdir() or member.isfile()):
                raise ValueError(f"runtime package contains unsafe link or device: {member.name}")
            member_path = (destination / member.name).resolve()
            if destination_resolved not in member_path.parents and member_path != destination_resolved:
                raise ValueError(f"runtime package contains unsafe path: {member.name}")
            if member_path in destinations:
                raise ValueError(f"runtime package contains duplicate path: {member.name}")
            destinations.add(member_path)
        package.extractall(destination)


def _safe_extract_zip(archive: Path, destination: Path) -> None:
    destination_resolved = destination.resolve()
    with zipfile.ZipFile(archive) as zipped:
        members = zipped.infolist()
        if len(members) > MAX_ARCHIVE_FILES:
            raise ValueError("artifact archive contains too many members")
        if sum(member.file_size for member in members) > MAX_UNPACKED_BYTES:
            raise ValueError("artifact archive exceeds the unpacked size limit")
        destinations: set[Path] = set()
        for member in members:
            member_path = (destination / member.filename).resolve()
            unix_mode = member.external_attr >> 16
            if stat.S_ISLNK(unix_mode):
                raise ValueError(f"artifact archive contains unsafe link: {member.filename}")
            if destination_resolved not in member_path.parents and member_path != destination_resolved:
                raise ValueError(f"artifact archive contains unsafe path: {member.filename}")
            if member_path in destinations:
                raise ValueError(f"artifact archive contains duplicate path: {member.filename}")
            destinations.add(member_path)
        zipped.extractall(destination)


def _github_token_for(url: str) -> str:
    if urlparse(url).hostname != "api.github.com":
        return ""
    return os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN") or ""


def _artifact_package(api_url: str, work: Path) -> Path:
    token = _github_token_for(api_url)
    metadata = work / "artifacts.json"
    _download(api_url, metadata, token)
    payload = json.loads(metadata.read_text(encoding="utf-8"))
    artifacts = payload.get("artifacts", []) if isinstance(payload, dict) else []
    selected = next(
        (
            item
            for item in artifacts
            if isinstance(item, dict)
            and not item.get("expired", True)
            and item.get("workflow_run", {}).get("head_branch") == "main"
            and isinstance(item.get("archive_download_url"), str)
        ),
        None,
    )
    if selected is None:
        raise RuntimeError("artifact source has no unexpired main-branch package")
    archive = work / "artifact.zip"
    download_url = selected["archive_download_url"]
    if token and urlparse(download_url).hostname not in {"api.github.com", "github.com"}:
        raise RuntimeError("artifact download URL is not a trusted GitHub host")
    _download(download_url, archive, token)
    artifact_root = work / "artifact"
    _safe_extract_zip(archive, artifact_root)
    matches = list(artifact_root.rglob("codexy-marketplace-plugin.tar.gz"))
    if len(matches) != 1:
        raise RuntimeError("artifact must contain exactly one marketplace package")
    return matches[0]


def acquire_package(
    *, path: str, url: str, artifacts_api: str, expected_sha256: str, work: Path
) -> Path:
    work.mkdir(parents=True, exist_ok=True)
    archive = work / "codexy-marketplace-plugin.tar.gz"
    if path:
        source = Path(path)
        if not source.is_absolute():
            raise ValueError(f"runtime package path must be absolute: {source}")
        shutil.copyfile(source, archive)
    elif url:
        _download(url, archive)
    elif artifacts_api:
        shutil.copyfile(_artifact_package(artifacts_api, work), archive)
    else:
        raise RuntimeError("no runtime package source was configured")
    if expected_sha256:
        observed = hashlib.sha256(archive.read_bytes()).hexdigest()
        if observed != expected_sha256.lower():
            raise ValueError(
                f"runtime package SHA-256 mismatch: expected {expected_sha256.lower()}, observed {observed}"
            )
    return archive


def unpack_runtime(*, archive: Path, work: Path, runtime_name: str) -> tuple[Path, Path]:
    extracted = work / "package"
    extracted.mkdir()
    _safe_extract_tar(archive, extracted)
    runtime = extracted / "plugins" / "codexy" / "runtime" / runtime_name
    manifest = extracted / "plugins" / "codexy" / ".codex-plugin" / "plugin.json"
    if not runtime.is_file() or runtime.is_symlink() or not manifest.is_file() or manifest.is_symlink():
        raise RuntimeError("runtime package is missing its exact runtime binary or plugin manifest")
    return runtime, manifest
