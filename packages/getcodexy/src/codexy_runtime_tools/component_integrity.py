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
        "agents/codexy-inspector.toml": "55a1080b3553a9da6aa42e3bc4fdba1f91690f29cf8afb937db874c77cb46002",
        "agents/codexy-sentinel.toml": "6b2868f888e967187107e4980939332135e8e62ceb9aa484d788e57aac316ac9",
        "agents/codexy-shipwright.toml": "f2f22cd0b857ae40aeb7d5a91d49653f2e87f48a752508f601308f0ac2cae508",
        "agents/codexy-warden.toml": "f58f37cedbc1f56ce8ba8a3ac2f60a55560d7903fc915d2df2457f0263b2db86",
        "skills/orchestration/scripts/agent_registration_fs.py": "7fb2a425b1e6fad29c99d7a56b4e8cef47faf3a098577bef2e8a5938931acdf5",
        "skills/orchestration/scripts/agent_registration_lifecycle.py": "814616c78beea769cb81dc5480f86f1176020ce188408d3d757b695a197d0804",
        "skills/orchestration/scripts/agent_registration_support.py": "6866b0f8d18a7910788ab8d5f8772f03d1e8b660dc9accdb5f6cbdd278e23e70",
        "skills/orchestration/scripts/register-codexy-agents": "f5b405a49525f9b66a735050f9ca3d22feb594ae42aeee94f480bb23ca3f4112",
    },
    "codexy-github": {
        "agents/catalog.toml": "a40af1007d226569b0856f8a1f64e022b473644092f355df21d9468e3107880d",
        "agents/codexy-weaver.toml": "2c88b22c48eb63400d207989e98a5919479737fba2cfb855992104217a0a2353",
        "skills/git-workflow/scripts/bootstrap-codexy-github-agent": "cc18f2a19e9784c6616c57a7d79d470e59b17cec6801b05bd94249d8c38dbedf",
    },
}

MANIFEST_CONTENT_DIGESTS = {
    "codexy": "d82c1adb0ae2804c10b5a9688671302b02f9a42798f0ec6eb7d17b6302e534ac",
    "codexy-github": "626de0d0be97ea6241d0353a92f94799b73b654b8469d0d6b7ae80a88e41b197",
}


def verify_component(root: Path, name: str, version: str | None = None) -> dict[Path, bytes]:
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
def frozen_component(root: Path, name: str, version: str | None = None) -> Iterator[Path]:
    contents = verify_component(root, name, version)
    with tempfile.TemporaryDirectory(prefix=f"{name}-verified-") as temporary:
        target = Path(temporary)
        for relative, data in contents.items():
            destination = target / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            descriptor = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(descriptor, "wb") as output:
                output.write(data)
        yield target


def _verify_manifest(contents: bytes, name: str, version: str | None) -> None:
    try:
        manifest = json.loads(contents, object_pairs_hook=_unique_object)
    except (json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"component manifest is invalid JSON: {name}") from error
    if not isinstance(manifest, dict) or (
        manifest.get("name"), manifest.get("repository")
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
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    directory_flags = flags | getattr(os, "O_DIRECTORY", 0)
    descriptor = os.open(root, directory_flags)
    try:
        for part in relative.parts[:-1]:
            next_descriptor = os.open(part, directory_flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        with os.fdopen(os.open(relative.name, flags, dir_fd=descriptor), "rb") as source:
            metadata = os.fstat(source.fileno())
            if not stat.S_ISREG(metadata.st_mode):
                raise ValueError(f"component integrity requires regular files: {root / relative}")
            return source.read()
    finally:
        os.close(descriptor)
