"""Installed-component health classification for read-only inspection."""

from __future__ import annotations

import json
import os
from pathlib import Path

from .component_manifest import ComponentManifest
from .component_registration_health import valid_registration
from .component_resolver import ComponentResolutionError, compare_versions


SURFACE_PATHS = {
    "core": ("agents/catalog.toml", "hooks/hooks.json", "skills/wiki/SKILL.md"),
    "github": ("agents/catalog.toml", "hooks/hooks.json"),
    "devtools": ("mcp/codexy-mcp-devtools", ".mcp.json"),
}


def health(
    manifest: ComponentManifest,
    actual: tuple[str, ...],
    recorded: tuple[str, ...] | None,
    records: dict[str, dict[str, object]],
    admission_error: str | None,
    host_error: bool,
) -> list[dict[str, str]]:
    expected, result = set(recorded or ()) | set(actual), []
    for component in manifest.component_ids:
        if component not in expected:
            continue
        record = records.get(component)
        if admission_error or host_error:
            result.append(entry(component, "incompatible"))
        elif component not in actual:
            result.append(entry(component, "missing"))
        elif version_relation(manifest, record) < 0:
            result.append(entry(component, "stale"))
        elif version_relation(manifest, record) > 0 or corrupt(
            manifest, component, record
        ):
            result.append(entry(component, "incompatible"))
        elif stale(manifest, component, record):
            result.append(entry(component, "stale"))
        elif not set(manifest.component(component).dependencies).issubset(actual):
            result.append(entry(component, "incompatible"))
        else:
            result.append({"component": component, "state": "healthy"})
    return result


def entry(component: str, state: str) -> dict[str, str]:
    repair = (
        "getcodexy bootstrap"
        if state in {"missing", "stale"}
        else "repair the Codexy registration, then rerun getcodexy doctor"
    )
    return {"component": component, "state": state, "repair": repair}


def version_relation(
    manifest: ComponentManifest, record: dict[str, object] | None
) -> int:
    version = record.get("version") if record else None
    try:
        return (
            compare_versions(version, manifest.version)
            if isinstance(version, str)
            else 1
        )
    except ComponentResolutionError:
        return 1


def stale(
    manifest: ComponentManifest, component: str, record: dict[str, object] | None
) -> bool:
    source = record.get("source") if record else None
    root = source.get("path") if isinstance(source, dict) else None
    if not isinstance(root, str) or not Path(root).is_absolute():
        return True
    plugin = Path(root)
    required = (
        manifest.component(component).asset.required_paths + SURFACE_PATHS[component]
    )
    if any(not regular(plugin / path) for path in required):
        return True
    if component == "core" and not wiki_is_valid(plugin / "skills/wiki/SKILL.md"):
        return True
    if component == "devtools" and not os.access(
        plugin / "mcp/codexy-mcp-devtools", os.X_OK
    ):
        return True
    return has_legacy_core_monolith(plugin, component)


def corrupt(
    manifest: ComponentManifest, component: str, record: dict[str, object] | None
) -> bool:
    source = record.get("source") if record else None
    root = source.get("path") if isinstance(source, dict) else None
    if not isinstance(root, str) or not Path(root).is_absolute():
        return False
    plugin = Path(root)
    required = (
        manifest.component(component).asset.required_paths + SURFACE_PATHS[component]
    )
    return all(regular(plugin / path) for path in required) and (
        not manifest_is_valid(
            plugin, manifest.component(component).plugin, record_version(record)
        )
        or not valid_registration(plugin, component)
    )


def record_version(record: dict[str, object] | None) -> str | None:
    version = record.get("version") if record else None
    return version if isinstance(version, str) else None


def regular(path: Path) -> bool:
    try:
        return path.is_file() and not path.is_symlink()
    except OSError:
        return False


def wiki_is_valid(path: Path) -> bool:
    try:
        return path.read_text(encoding="utf-8").startswith("---\n")
    except (OSError, UnicodeDecodeError):
        return False


def manifest_is_valid(plugin: Path, name: str, version: str | None) -> bool:
    value = json_value(plugin / ".codex-plugin/plugin.json")
    return (
        isinstance(value, dict)
        and value.get("name") == name
        and value.get("repository") == "https://github.com/eunsoogi/codexy"
        and version is not None
        and value.get("version") == version
    )


def has_legacy_core_monolith(plugin: Path, component: str) -> bool:
    return component == "core" and any(
        os.path.lexists(plugin / path)
        for path in (
            ".mcp.json",
            ".codex/lsp-client.json",
            "lsp",
            "mcp",
            "runtime-release.json",
        )
    )


def json_value(path: Path) -> object | None:
    try:
        with path.open("r", encoding="utf-8") as source:
            return json.load(source)
    except (OSError, UnicodeDecodeError, ValueError, json.JSONDecodeError):
        return None
