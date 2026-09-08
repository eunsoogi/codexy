"""Validation for the optional core Watcher runtime class."""

from typing import Any

from .identity import CANDIDATE_PLATFORMS, digest, object


def validate_watcher(value: Any) -> dict[str, Any]:
    watcher = object(value, "coreWatcherMcp")
    if set(watcher) != {"platforms"}:
        raise ValueError("runtime release core watcher has unknown or missing fields")
    binaries = object(watcher.get("platforms"), "core watcher platforms")
    if set(binaries) != CANDIDATE_PLATFORMS:
        raise ValueError(
            "runtime release core watcher must cover all candidate platforms"
        )
    for platform, binary in binaries.items():
        binary = object(binary, f"core watcher platforms.{platform}")
        if set(binary) != {"path", "sha256", "kind"}:
            raise ValueError(
                "runtime release core watcher binary has unknown or missing fields"
            )
        extension = "exe" if platform == "windows-x86_64" else "bin"
        if binary.get("path") != f"runtime/codexy-mcp-watcher-{platform}.{extension}":
            raise ValueError("runtime release core watcher path is not canonical")
        digest(binary.get("sha256"), "core watcher.sha256")
        expected_kind = {
            "darwin-arm64": "mach-o",
            "linux-x86_64": "elf",
            "windows-x86_64": "pe",
        }[platform]
        if binary.get("kind") != expected_kind:
            raise ValueError("runtime release core watcher kind is not canonical")
    return watcher
