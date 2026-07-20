from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path


SEMVER = re.compile(
    r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-((?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)


def plugin_release(manifest_path: Path, package_override: bool = False) -> str:
    if not manifest_path.is_file():
        if package_override:
            return "package-override"
        raise ValueError("plugin manifest is missing")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    release = manifest.get("version") if isinstance(manifest, dict) else None
    if not isinstance(release, str) or not SEMVER.fullmatch(release):
        raise ValueError("plugin manifest version is invalid")
    return release


def releases_match(expected_manifest: Path, observed_manifest: Path) -> tuple[bool, str]:
    try:
        expected = plugin_release(expected_manifest)
    except (OSError, ValueError, json.JSONDecodeError):
        return False, "runtime package release mismatch: expected valid plugin release, observed missing or invalid"
    try:
        observed = plugin_release(observed_manifest)
    except (OSError, ValueError, json.JSONDecodeError):
        return False, f"runtime package release mismatch: expected {expected}, observed missing or invalid"
    if expected != observed:
        return False, f"runtime package release mismatch: expected {expected}, observed {observed}"
    return True, ""


def runtime_cache_key(*, manifest: Path, package_override: bool, identity: list[str]) -> str:
    release = plugin_release(manifest, package_override)
    digest_input = "\0".join(("codexy.runtime-cache/v2", *identity[:-1], release, identity[-1]))
    return f"v2-{hashlib.sha256(digest_input.encode()).hexdigest()}"
