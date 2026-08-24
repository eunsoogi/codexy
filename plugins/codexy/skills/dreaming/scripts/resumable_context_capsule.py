#!/usr/bin/env python3
"""Authenticate the installed native handoff bridge and invoke it."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import stat
import subprocess
import sys
import tempfile
from pathlib import Path

SCHEMA = "codexy.handoff-runtime.v1"
PLATFORMS = ("darwin-arm64", "linux-x86_64", "windows-x86_64")
KINDS = {
    "darwin-arm64": "mach-o",
    "linux-x86_64": "elf",
    "windows-x86_64": "pe",
}


def fail(message: str) -> None:
    raise SystemExit(message)


def absolute(path: Path) -> Path:
    return Path(os.path.abspath(path))


def reparse(metadata: os.stat_result) -> bool:
    attribute = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return bool(getattr(metadata, "st_file_attributes", 0) & attribute)


def safe_ancestors(path: Path, *, directory: bool, label: str) -> os.stat_result:
    path = absolute(path)
    existing = path if path.exists() or path.is_symlink() else path.parent
    chain = [existing, *existing.parents]
    for ancestor in reversed(chain):
        metadata = ancestor.lstat()
        if stat.S_ISLNK(metadata.st_mode) or reparse(metadata):
            fail(f"{label} ancestor must not be linked or reparse: {ancestor}")
    metadata = path.lstat()
    expected = (
        stat.S_ISDIR(metadata.st_mode) if directory else stat.S_ISREG(metadata.st_mode)
    )
    if not expected:
        fail(
            f"{label} must be a {'directory' if directory else 'regular file'}: {path}"
        )
    return metadata


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate manifest field: {key}")
        result[key] = value
    return result


def exact_keys(value: object, expected: set[str], label: str) -> dict[str, object]:
    if not isinstance(value, dict) or set(value) != expected:
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


def platform_id() -> str:
    systems = {"Darwin": "darwin", "Linux": "linux", "Windows": "windows"}
    machines = {
        "arm64": "arm64",
        "aarch64": "arm64",
        "x86_64": "x86_64",
        "AMD64": "x86_64",
    }
    try:
        return f"{systems[platform.system()]}-{machines[platform.machine()]}"
    except KeyError as error:
        fail(f"unsupported selected platform: {platform.system()}-{platform.machine()}")
        raise AssertionError from error


def default_runtime_root(plugin_root: Path) -> Path:
    root = (
        plugin_root
        if (plugin_root / "handoff-runtime.json").is_file()
        else plugin_root.parent / "codexy-devtools"
    )
    safe_ancestors(root, directory=True, label="runtime")
    return root


def load_manifest(root: Path) -> tuple[dict[str, object], Path]:
    path = root / "handoff-runtime.json"
    safe_ancestors(path, directory=False, label="authority")
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=unique_object
        )
    except (OSError, json.JSONDecodeError) as error:
        fail(f"invalid handoff runtime manifest: {error}")
    manifest = exact_keys(
        value, {"schema", "version", "source", "platforms"}, "manifest"
    )
    if manifest["schema"] != SCHEMA or manifest["version"] != 1:
        fail("manifest schema or version mismatch")
    source = exact_keys(manifest["source"], {"commit", "tree"}, "source")
    lower_hex(source["commit"], 40, "source commit")
    lower_hex(source["tree"], 40, "source tree")
    platforms = exact_keys(manifest["platforms"], set(PLATFORMS), "platforms")
    return platforms, path


def selected_bridge(root: Path, platforms: dict[str, object]) -> Path:
    selected = platform_id()
    item = exact_keys(
        platforms[selected], {"path", "sha256", "kind"}, "selected platform"
    )
    suffix = ".exe" if selected == "windows-x86_64" else ".bin"
    expected_path = f"runtime/codexy-handoff-validate-{selected}{suffix}"
    if item["path"] != expected_path:
        fail("selected platform path mismatch")
    if item["kind"] != KINDS[selected]:
        fail("selected platform kind mismatch")
    digest = lower_hex(item["sha256"], 64, "selected platform digest")
    bridge = root / expected_path
    metadata = safe_ancestors(bridge, directory=False, label="native bridge")
    if os.name != "nt" and metadata.st_mode & 0o111 == 0:
        fail("selected platform bridge is not executable")
    if hashlib.sha256(bridge.read_bytes()).hexdigest() != digest:
        fail("selected platform digest mismatch")
    expected_magic = {
        "elf": b"\x7fELF",
        "mach-o": (b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf"),
        "pe": b"MZ",
    }[item["kind"]]
    prefix = bridge.read_bytes()[:4]
    if isinstance(expected_magic, tuple):
        valid_kind = prefix in expected_magic
    else:
        valid_kind = prefix.startswith(expected_magic)
    if not valid_kind:
        fail("selected platform kind does not match bridge bytes")
    return bridge


def invoke(bridge: Path, capsule: Path, authority: Path, output: Path | None) -> int:
    safe_ancestors(capsule, directory=False, label="capsule")
    safe_ancestors(authority, directory=False, label="authority")
    if output is not None:
        safe_ancestors(output.parent, directory=True, label="output")
    result = subprocess.run(
        [bridge, "--capsule", capsule, "--authority", authority],
        capture_output=True,
        check=False,
    )
    if result.returncode:
        sys.stderr.buffer.write(result.stderr)
        return result.returncode
    if output is None:
        sys.stdout.buffer.write(result.stdout)
    else:
        descriptor, temporary = tempfile.mkstemp(prefix=".capsule-", dir=output.parent)
        try:
            with os.fdopen(descriptor, "wb") as destination:
                destination.write(result.stdout)
                destination.flush()
                os.fsync(destination.fileno())
            os.replace(temporary, output)
        finally:
            if os.path.exists(temporary):
                os.unlink(temporary)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--capsule", required=True, type=Path)
    parser.add_argument("--authority", required=True, type=Path)
    parser.add_argument("--runtime-root", type=Path)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    plugin_root = Path(__file__).resolve().parents[3]
    runtime_root = (
        absolute(arguments.runtime_root)
        if arguments.runtime_root
        else default_runtime_root(plugin_root)
    )
    safe_ancestors(runtime_root, directory=True, label="runtime")
    platforms, _ = load_manifest(runtime_root)
    return invoke(
        selected_bridge(runtime_root, platforms),
        arguments.capsule,
        arguments.authority,
        arguments.output,
    )


if __name__ == "__main__":
    raise SystemExit(main())
