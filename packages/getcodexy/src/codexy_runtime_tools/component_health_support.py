"""Shared file and provenance checks for component health reports."""

import json
import os
from pathlib import Path

from .component_manifest import ComponentManifest
from .component_registration_health import valid_registration
from .component_resolver import ComponentResolutionError, compare_versions
from .component_watcher_materialization import (
    valid_watcher_cache,
    watcher_cache_plugin,
    watcher_entrypoint,
)


SURFACE_PATHS = {
    "core": (
        "agents/catalog.toml",
        "hooks/hooks.json",
        "skills/wiki/SKILL.md",
        ".mcp.json",
        "mcp/codexy-mcp-watcher.sh",
        "mcp/codexy-mcp-watcher.cmd",
    ),
    "github": ("agents/catalog.toml", "hooks/hooks.json"),
    "devtools": ("mcp/codexy-mcp-devtools", ".mcp.json"),
}
AUTHORITY_KEYS = ("authority", "artifact_authority", "artifactAuthority")


def _legacy_state(
    manifest,
    component,
    actual,
    records,
    admission_error,
    host_error,
    codex_home=None,
):
    record = records.get(component)
    if admission_error or host_error:
        return "incompatible"
    if component not in actual:
        return "missing"
    relation = version_relation(manifest, record)
    if relation != 0:
        return "stale" if relation < 0 else "incompatible"
    plugin = _health_plugin(manifest, component, record, codex_home)
    if plugin is None or not _required_files(manifest, component, plugin, codex_home):
        return "stale"
    if not manifest_is_valid(
        plugin, manifest.component(component).plugin, record_version(record)
    ) or not valid_registration(plugin, component):
        return "incompatible"
    if component == "devtools" and not os.access(
        plugin / "mcp/codexy-mcp-devtools", os.X_OK
    ):
        return "stale"
    if component == "core" and any(
        os.path.lexists(plugin / path)
        for path in (".codex/lsp-client.json", "lsp", "runtime-release.json")
    ):
        return "stale"
    if not set(manifest.component(component).dependencies).issubset(actual):
        return "incompatible"
    return "healthy"


def _required_files(manifest, component, plugin, codex_home=None):
    paths = (
        manifest.component(component).asset.required_paths + SURFACE_PATHS[component]
    )
    return all(
        (plugin / path).is_file() and not (plugin / path).is_symlink()
        for path in paths
    ) and (
        component != "core"
        or (
            watcher_entrypoint(plugin).is_file()
            and valid_watcher_cache(codex_home, manifest.version)
        )
    )


def _health_plugin(manifest, component, record, codex_home=None):
    plugin = _plugin_root(record)
    if component != "core" or codex_home is None:
        return plugin
    cache_root = codex_home / "plugins" / "cache"
    if cache_root.is_symlink() or not cache_root.is_dir():
        return plugin
    return watcher_cache_plugin(codex_home, manifest.version)


def _plugin_root(record):
    source = (record or {}).get("source")
    value = source.get("path") if isinstance(source, dict) else None
    return Path(value) if isinstance(value, str) and Path(value).is_absolute() else None


def _observed(record):
    return dict(
        plugin=dict(name=(record or {}).get("name"), version=record_version(record)),
        runtime=dict(name=None, version=None),
    )


def _authority_valid(record):
    if not record:
        return False
    key = next((key for key in AUTHORITY_KEYS if key in record), None)
    if key is None:
        if record.get("marketplaceSource") == {
            "sourceType": "git",
            "source": "https://github.com/eunsoogi/codexy.git",
        }:
            return True
        installed_source = record.get("source")
        marketplace_source = record.get("marketplaceSource")
        if not (
            isinstance(installed_source, dict)
            and isinstance(installed_source.get("path"), str)
            and isinstance(marketplace_source, dict)
            and marketplace_source.get("sourceType") == "local"
            and isinstance(marketplace_source.get("source"), str)
        ):
            return False
        path, root = Path(installed_source["path"]), Path(marketplace_source["source"])
        return (
            path.is_absolute()
            and root.is_absolute()
            and path.parent.name == "plugins"
            and path.parent.parent == root
        )
    authority = record[key]
    return isinstance(authority, dict) and authority.get("state") in {
        "valid",
        "attested",
    }


def version_relation(manifest, record):
    try:
        return compare_versions(record_version(record), manifest.version)
    except ComponentResolutionError:
        return 1


def record_version(record):
    value = record.get("version") if record else None
    return value if isinstance(value, str) else None


def manifest_is_valid(plugin, name, version):
    value = json_value(plugin / ".codex-plugin/plugin.json")
    return (
        version is not None
        and isinstance(value, dict)
        and (value.get("name"), value.get("repository"), value.get("version"))
        == (name, "https://github.com/eunsoogi/codexy", version)
    )


def json_value(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, ValueError):
        return None
