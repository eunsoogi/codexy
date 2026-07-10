#!/usr/bin/env python3
"""Build versioned cache keys for packaged Codexy MCP runtimes."""

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
