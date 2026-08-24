#!/usr/bin/env python3
"""Validate the closed generated core-handoff runtime manifest."""

from __future__ import annotations

import hashlib
import json
import os
import stat
import sys
from pathlib import Path

PLATFORMS = ("darwin-arm64", "linux-x86_64", "windows-x86_64")
KINDS = {"darwin-arm64": "mach-o", "linux-x86_64": "elf", "windows-x86_64": "pe"}


def fail(message: str) -> None:
    raise SystemExit(message)


def exact(value: object, fields: set[str], label: str) -> dict:
    if not isinstance(value, dict) or set(value) != fields:
        fail(f"{label} fields are not closed")
    return value


def lower_hex(value: object, length: int, label: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) != length
        or any(character not in "0123456789abcdef" for character in value)
    ):
        fail(f"{label} must be lowercase hexadecimal")
    return value


def safe_file(root: Path, relative: str) -> Path:
    path = root / relative
    current = root
    for part in Path(relative).parts:
        if part in {"", ".", ".."}:
            fail("bridge path is not relative and bounded")
        current /= part
        metadata = current.lstat()
        attribute = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
        if stat.S_ISLNK(metadata.st_mode) or bool(
            getattr(metadata, "st_file_attributes", 0) & attribute
        ):
            fail(f"bridge ancestor must not be linked or reparse: {current}")
    if not path.is_file():
        fail(f"bridge must be a regular file: {relative}")
    return path


def validate(manifest_path: Path, root: Path) -> dict:
    value = json.loads(manifest_path.read_text(), object_pairs_hook=unique)
    manifest = exact(value, {"schema", "version", "source", "platforms"}, "manifest")
    if manifest["schema"] != "codexy.handoff-runtime.v1" or manifest["version"] != 1:
        fail("manifest schema or version mismatch")
    source = exact(manifest["source"], {"commit", "tree"}, "source")
    lower_hex(source["commit"], 40, "source commit")
    lower_hex(source["tree"], 40, "source tree")
    platforms = exact(manifest["platforms"], set(PLATFORMS), "platforms")
    for platform in PLATFORMS:
        bridge = exact(platforms[platform], {"path", "sha256", "kind"}, platform)
        extension = "exe" if platform == "windows-x86_64" else "bin"
        expected = f"runtime/codexy-handoff-validate-{platform}.{extension}"
        if bridge["path"] != expected:
            fail(f"{platform} path mismatch")
        if bridge["kind"] != KINDS[platform]:
            fail(f"{platform} executable kind mismatch")
        digest = lower_hex(bridge["sha256"], 64, f"{platform} digest")
        path = safe_file(root, expected)
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            fail(f"{platform} digest mismatch")
        if (
            os.name != "nt"
            and platform != "windows-x86_64"
            and path.stat().st_mode & 0o111 == 0
        ):
            fail(f"{platform} bridge is not executable")
    return manifest


def unique(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate manifest field: {key}")
        result[key] = value
    return result


def main() -> None:
    if len(sys.argv) != 3:
        fail("usage: handoff_runtime_contract.py MANIFEST RUNTIME_ROOT")
    validate(Path(sys.argv[1]), Path(sys.argv[2]))


if __name__ == "__main__":
    main()
