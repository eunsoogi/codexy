#!/usr/bin/env python3
"""Build versioned cache keys and compare package releases for MCP runtimes.

Implicit release and artifact packages must match the installed plugin release.
Explicit package-source overrides use a separate cache key and intentionally skip
that comparison so callers can test or pin a package independently.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path


SEMVER = re.compile(
    r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-((?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)


def plugin_release(manifest_path: str, package_override: str) -> str:
    path = Path(manifest_path)
    if not path.is_file():
        if package_override == "1":
            return "package-override"
        raise ValueError("plugin manifest is missing")
    with path.open(encoding="utf-8") as manifest_file:
        manifest = json.load(manifest_file)
    release = manifest.get("version") if isinstance(manifest, dict) else None
    if not isinstance(release, str) or not SEMVER.fullmatch(release):
        raise ValueError("plugin manifest version is invalid")
    return release


def main(arguments: list[str]) -> int:
    if len(arguments) == 3 and arguments[0] == "--compare-release":
        _, expected_manifest, observed_manifest = arguments
        try:
            expected = plugin_release(expected_manifest, "0")
        except (OSError, ValueError, json.JSONDecodeError):
            print("runtime package release mismatch: expected valid plugin release, observed missing or invalid")
            return 1
        try:
            observed = plugin_release(observed_manifest, "0")
        except (OSError, ValueError, json.JSONDecodeError):
            print(f"runtime package release mismatch: expected {expected}, observed missing or invalid")
            return 1
        if expected != observed:
            print(f"runtime package release mismatch: expected {expected}, observed {observed}")
            return 1
        return 0
    if len(arguments) != 8:
        return 2
    manifest_path, package_override, *identity = arguments
    try:
        release = plugin_release(manifest_path, package_override)
    except (OSError, ValueError, json.JSONDecodeError):
        return 1
    digest_input = "\0".join(("codexy.runtime-cache/v2", *identity[:-1], release, identity[-1]))
    print(f"v2-{hashlib.sha256(digest_input.encode()).hexdigest()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
