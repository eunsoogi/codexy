"""Pinned package-content integrity for public component activation."""

from __future__ import annotations

import hashlib
import json
import os
import stat
import tempfile
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

from .updater import _absolute, _validate_real_path


COMPONENT_FILES = {
    "codexy": {
        "agents/catalog.toml": "a082c51976773ee5ec2e1f1a6589b48b6f28c6309298236ffd165ddb9c6e1858",
        "agents/codexy-architect.toml": "836531b20493580d8c89628ffe49d198c54c92e8b6c5eb4eab2d907edfde015a",
        "agents/codexy-auditor.toml": "98929f2e94d9562fbc6ea7b54c97ad8a2769ce1bee1d769619147ab2bd0427f0",
        "agents/codexy-cartographer.toml": "459496f918693ff574e6913be95ed601c78187c3742d4d00e4a88e979487641f",
        "agents/codexy-inspector.toml": "183e7db642ebad22feed3cddc1358c954a953fe2fc296836bddd9ff4d85d9bfd",
        "agents/codexy-sentinel.toml": "b6d724656084ca8b2200d80b2e47121485052360d9d4801df6a7f019c5dcb82b",
        "agents/codexy-shipwright.toml": "f2f22cd0b857ae40aeb7d5a91d49653f2e87f48a752508f601308f0ac2cae508",
        "agents/codexy-warden.toml": "f58f37cedbc1f56ce8ba8a3ac2f60a55560d7903fc915d2df2457f0263b2db86",
        "skills/orchestration/scripts/agent_registration_blocks.py": "d9fee4e722e6595a29aa038d3db1404f134763c80df618593f82ecc54089069b",
        "skills/orchestration/scripts/agent_registration_fs.py": "c5f1952770d4c83d662a719d24a7d30da7a266c105f9b981b99d730a8c03298e",
        "skills/orchestration/scripts/agent_registration_lifecycle.py": "9b1762d6fa066ac118c04ca61e6181997b84bf7e924ebf255703954f4e25e871",
        "skills/orchestration/scripts/agent_registration_support.py": "6aeae4d9107de34d9b79cb4c3e8898d0129b0e1f74fa57bf0825f34dd940371f",
        "skills/orchestration/scripts/register_codexy_agents.py": "3364d7bae75c351ce89aea4cbfadb46dab6260854db76851a2f13559cd8ccd7d",
    },
    "codexy-github": {
        "agents/catalog.toml": "a40af1007d226569b0856f8a1f64e022b473644092f355df21d9468e3107880d",
        "agents/codexy-weaver.toml": "2c88b22c48eb63400d207989e98a5919479737fba2cfb855992104217a0a2353",
        "skills/git-workflow/scripts/bootstrap_codexy_github_agent.py": "49983a120fd999ffc0e47e1211ecbdcb5e94a838c8c5abe833f6f4b9fb39363f",
    },
}

MANIFEST_CONTENT_DIGESTS = {
    "codexy": "7f1cfa40bc4fda532de26d396d9cbe41aa5966a3549951f124da8a481160b8dd",
    "codexy-github": "626de0d0be97ea6241d0353a92f94799b73b654b8469d0d6b7ae80a88e41b197",
}


def verify_component(
    root: Path, name: str, version: str | None = None
) -> dict[Path, bytes]:
    try:
        expected = COMPONENT_FILES[name]
    except KeyError as error:
        raise ValueError(f"unknown component integrity identity: {name}") from error
    root = _absolute(root)
    _validate_real_path(root, require_exists=True)
    verified = {}
    for source, digest in expected.items():
        relative = Path(source)
        contents = _read_regular(root, relative)
        observed = hashlib.sha256(contents).hexdigest()
        if observed != digest:
            raise ValueError(f"component integrity mismatch: {root / relative}")
        verified[relative] = contents
    manifest = Path(".codex-plugin/plugin.json")
    manifest_contents = _read_regular(root, manifest)
    _verify_manifest(manifest_contents, name, version)
    verified[manifest] = manifest_contents
    return verified


@contextmanager
def frozen_component(
    root: Path, name: str, version: str | None = None
) -> Iterator[Path]:
    contents = verify_component(root, name, version)
    with tempfile.TemporaryDirectory(prefix=f"{name}-verified-") as temporary:
        target = Path(temporary)
        for relative, data in contents.items():
            destination = target / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            descriptor = os.open(
                destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600
            )
            with os.fdopen(descriptor, "wb") as output:
                output.write(data)
        yield target


def _verify_manifest(contents: bytes, name: str, version: str | None) -> None:
    try:
        manifest = json.loads(contents, object_pairs_hook=_unique_object)
    except (json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"component manifest is invalid JSON: {name}") from error
    if not isinstance(manifest, dict) or (
        manifest.get("name"),
        manifest.get("repository"),
    ) != (name, "https://github.com/eunsoogi/codexy"):
        raise ValueError(f"component manifest identity mismatch: {name}")
    if version is not None and manifest.get("version") != version:
        raise ValueError(f"component manifest version mismatch: {name}")
    normalized = dict(manifest, version="<VERSION>")
    observed = hashlib.sha256(
        json.dumps(normalized, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    if observed != MANIFEST_CONTENT_DIGESTS[name]:
        raise ValueError(f"component manifest content mismatch: {name}")


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _read_regular(root: Path, relative: Path) -> bytes:
    if _uses_windows_directory_fallback():
        return _read_regular_windows(root, relative)
    return _read_regular_posix(root, relative)


def _uses_windows_directory_fallback() -> bool:
    return os.name == "nt"


def _read_regular_posix(root: Path, relative: Path) -> bytes:
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    directory_flags = flags | getattr(os, "O_DIRECTORY", 0)
    descriptor = os.open(root, directory_flags)
    try:
        for part in relative.parts[:-1]:
            next_descriptor = os.open(part, directory_flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        with os.fdopen(
            os.open(relative.name, flags, dir_fd=descriptor), "rb"
        ) as source:
            metadata = os.fstat(source.fileno())
            if not stat.S_ISREG(metadata.st_mode):
                raise ValueError(
                    f"component integrity requires regular files: {root / relative}"
                )
            return source.read()
    finally:
        os.close(descriptor)


def _read_regular_windows(root: Path, relative: Path) -> bytes:
    target, _ = _windows_safe_path(root, relative)
    descriptor = os.open(target, os.O_RDONLY | getattr(os, "O_BINARY", 0))
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode):
            raise ValueError(f"component integrity requires regular files: {target}")
        _, final = _windows_safe_path(root, relative)
        if (opened.st_dev, opened.st_ino) != (final.st_dev, final.st_ino):
            raise ValueError(
                f"component integrity path changed while reading: {target}"
            )
        with os.fdopen(descriptor, "rb", closefd=False) as source:
            return source.read()
    finally:
        os.close(descriptor)


def _windows_safe_path(root: Path, relative: Path) -> tuple[Path, os.stat_result]:
    if relative.is_absolute() or any(
        part in {"", ".", ".."} for part in relative.parts
    ):
        raise ValueError(f"component integrity path is invalid: {relative}")
    current = root
    _windows_regular_path(current, directory=True)
    for part in relative.parts[:-1]:
        current /= part
        _windows_regular_path(current, directory=True)
    target = current / relative.name
    return target, _windows_regular_path(target, directory=False)


def _windows_regular_path(path: Path, directory: bool) -> os.stat_result:
    metadata = path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or _has_windows_reparse_point(metadata):
        raise ValueError(
            f"component integrity path must not traverse link or reparse point: {path}"
        )
    if stat.S_ISDIR(metadata.st_mode) != directory:
        kind = "directory" if directory else "regular file"
        raise ValueError(f"component integrity requires {kind}: {path}")
    return metadata


def _has_windows_reparse_point(metadata: os.stat_result) -> bool:
    attribute = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return bool(getattr(metadata, "st_file_attributes", 0) & attribute)
