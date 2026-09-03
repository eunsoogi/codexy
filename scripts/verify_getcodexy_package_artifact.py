#!/usr/bin/env python3
"""Validate the getcodexy wheel and source distribution payloads."""

from __future__ import annotations

import argparse
import json
import re
import tarfile
from pathlib import Path
from zipfile import ZipFile


MANIFEST_SUFFIX = "codexy_runtime_tools/component-manifest.json"
VERSION_RE = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument("--version", required=True)
    arguments = parser.parse_args()
    if VERSION_RE.fullmatch(arguments.version) is None:
        raise SystemExit("package version must be MAJOR.MINOR.PATCH")
    directory = arguments.dist
    if not directory.is_dir():
        raise SystemExit(f"package distribution directory is missing: {directory}")
    wheels = sorted(directory.glob("*.whl"))
    sdists = sorted(directory.glob("*.tar.gz"))
    if len(wheels) != 1 or len(sdists) != 1:
        raise SystemExit(
            f"expected exactly one wheel and sdist, got wheels={len(wheels)} sdists={len(sdists)}"
        )
    _verify_wheel(wheels[0], arguments.version)
    _verify_sdist(sdists[0], arguments.version)
    print(
        f"getcodexy package payload verified: version={arguments.version} "
        f"wheel={wheels[0].name} sdist={sdists[0].name}"
    )
    return 0


def _verify_wheel(path: Path, expected: str) -> None:
    with ZipFile(path) as archive:
        names = archive.namelist()
        manifests = [name for name in names if name.endswith(MANIFEST_SUFFIX)]
        if len(manifests) != 1:
            raise SystemExit(f"wheel must contain one component manifest: {path}")
        metadata = [name for name in names if name.endswith(".dist-info/METADATA")]
        if len(metadata) != 1:
            raise SystemExit(f"wheel must contain one dist-info METADATA: {path}")
        _verify_metadata(archive.read(metadata[0]), expected, "wheel")
        _verify_manifest(archive.read(manifests[0]), expected, "wheel")


def _verify_sdist(path: Path, expected: str) -> None:
    with tarfile.open(path, "r:gz") as archive:
        members = [member for member in archive.getmembers() if member.isfile()]
        manifests = [
            member for member in members if member.name.endswith(MANIFEST_SUFFIX)
        ]
        if len(manifests) != 1:
            raise SystemExit(f"sdist must contain one component manifest: {path}")
        metadata = [member for member in members if member.name.endswith("/PKG-INFO")]
        if len(metadata) != 1:
            raise SystemExit(f"sdist must contain one PKG-INFO: {path}")
        _verify_metadata(archive.extractfile(metadata[0]).read(), expected, "sdist")
        _verify_manifest(archive.extractfile(manifests[0]).read(), expected, "sdist")


def _verify_metadata(raw: bytes, expected: str, artifact: str) -> None:
    version = next(
        (
            line.removeprefix("Version: ").strip()
            for line in raw.decode().splitlines()
            if line.startswith("Version: ")
        ),
        None,
    )
    if version != expected:
        raise SystemExit(f"{artifact} package metadata version mismatch: {version!r}")


def _verify_manifest(raw: bytes, expected: str, artifact: str) -> None:
    try:
        manifest = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise SystemExit(f"{artifact} component manifest is not valid JSON") from error
    components = manifest.get("components") if isinstance(manifest, dict) else None
    combinations = (
        manifest.get("compatibleCombinations") if isinstance(manifest, dict) else None
    )
    versions = (
        [item.get("version") for item in components if isinstance(item, dict)]
        if isinstance(components, list)
        else []
    )
    combination_versions = (
        [item.get("version") for item in combinations if isinstance(item, dict)]
        if isinstance(combinations, list)
        else []
    )
    if (
        not isinstance(manifest, dict)
        or manifest.get("schema") != "getcodexy.component-manifest.v1"
        or len(components or ()) != 3
        or len(combination_versions) == 0
        or len(versions) != 3
        or any(version != expected for version in versions + combination_versions)
    ):
        raise SystemExit(f"{artifact} component manifest version mismatch")


if __name__ == "__main__":
    raise SystemExit(main())
