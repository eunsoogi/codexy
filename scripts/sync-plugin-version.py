#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
PLUGIN_NAME = "codexy"
PLUGIN_MANIFEST = REPO_ROOT / "plugins" / PLUGIN_NAME / ".codex-plugin" / "plugin.json"
MARKETPLACE = REPO_ROOT / ".agents" / "plugins" / "marketplace.json"
PACKAGE_MANIFESTS = [REPO_ROOT / "package.json"]
SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:[-+][0-9A-Za-z.-]+)?$")


class VersionError(RuntimeError):
    pass


def load_json(path: Path) -> Any:
    try:
        with path.open(encoding="utf-8") as handle:
            return json.load(handle)
    except FileNotFoundError as exc:
        raise VersionError(f"missing required file: {path.relative_to(REPO_ROOT)}") from exc
    except json.JSONDecodeError as exc:
        raise VersionError(f"invalid JSON in {path.relative_to(REPO_ROOT)}: {exc}") from exc


def write_json(path: Path, data: Any) -> None:
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def require_semver(version: str) -> None:
    if not SEMVER_RE.match(version):
        raise VersionError(f"version must be semver-like MAJOR.MINOR.PATCH: {version!r}")


def marketplace_plugin(marketplace: dict[str, Any]) -> dict[str, Any]:
    plugins = marketplace.get("plugins")
    if not isinstance(plugins, list):
        raise VersionError(".agents/plugins/marketplace.json must contain a plugins array")

    matches = [plugin for plugin in plugins if isinstance(plugin, dict) and plugin.get("name") == PLUGIN_NAME]
    if len(matches) != 1:
        raise VersionError(f"expected exactly one marketplace plugin named {PLUGIN_NAME!r}, found {len(matches)}")
    return matches[0]


def package_manifests() -> list[tuple[Path, dict[str, Any], str]]:
    packages: list[tuple[Path, dict[str, Any], str]] = []
    for path in PACKAGE_MANIFESTS:
        if not path.exists():
            continue
        data = load_json(path)
        version = data.get("version")
        if not isinstance(version, str):
            raise VersionError(f"{path.relative_to(REPO_ROOT)} version must be a string")
        require_semver(version)
        packages.append((path, data, version))
    return packages


def collect_versions() -> tuple[dict[str, Any], dict[str, Any], str, str, list[tuple[Path, dict[str, Any], str]]]:
    manifest = load_json(PLUGIN_MANIFEST)
    marketplace = load_json(MARKETPLACE)

    manifest_version = manifest.get("version")
    if not isinstance(manifest_version, str):
        raise VersionError("plugin manifest version must be a string")
    require_semver(manifest_version)

    entry = marketplace_plugin(marketplace)
    marketplace_version = entry.get("version")
    if not isinstance(marketplace_version, str):
        raise VersionError("marketplace plugin entry must include a string version")
    require_semver(marketplace_version)

    return manifest, marketplace, manifest_version, marketplace_version, package_manifests()


def check_versions() -> None:
    _, _, manifest_version, marketplace_version, packages = collect_versions()
    if manifest_version != marketplace_version:
        raise VersionError(
            "version mismatch: "
            f"{PLUGIN_MANIFEST.relative_to(REPO_ROOT)}={manifest_version}, "
            f"{MARKETPLACE.relative_to(REPO_ROOT)}={marketplace_version}"
        )
    for path, _, package_version in packages:
        if package_version != manifest_version:
            raise VersionError(
                "version mismatch: "
                f"{path.relative_to(REPO_ROOT)}={package_version}, "
                f"{PLUGIN_MANIFEST.relative_to(REPO_ROOT)}={manifest_version}"
            )
    print(f"plugin version sync ok: {manifest_version}")


def set_version(version: str) -> None:
    require_semver(version)
    manifest, marketplace, _, _, packages = collect_versions()

    manifest["version"] = version
    marketplace_plugin(marketplace)["version"] = version

    write_json(PLUGIN_MANIFEST, manifest)
    write_json(MARKETPLACE, marketplace)
    for path, data, _ in packages:
        data["version"] = version
        write_json(path, data)
    print(f"plugin version synchronized to {version}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check or synchronize Codexy plugin version metadata.")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true", help="verify all plugin version fields match")
    mode.add_argument("--version", help="set all plugin version fields to this version")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.check:
            check_versions()
        else:
            set_version(args.version)
    except VersionError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
