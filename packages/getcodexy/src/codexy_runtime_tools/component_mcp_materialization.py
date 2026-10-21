"""Shared per-component MCP bootstrap and source/cache consistency checks."""

from __future__ import annotations

import json
import os
from pathlib import Path

from .component_mcp_cache import _require_directory
from .component_mcp_capability import (
    MCP_SPECS,
    McpSpec,
    mcp_configuration,
    mcp_spec,
    selected_mcp_spec,
    sync_mcp_surface,
    valid_mcp_surface,
)
from .updater import _absolute, _validate_real_path


REPOSITORY = "https://github.com/eunsoogi/codexy"
CACHE_ROOT = Path("plugins/cache/codexy")
MCP_COMPONENTS = frozenset(MCP_SPECS)


def materialize_component_mcp(
    plugin: Path, component: str, version: str | None = None
) -> Path:
    """Validate the selected package contract before its host installation."""
    root = _plugin_root(plugin)
    selected_version = _plugin_version(root, component)
    if version is not None and selected_version != version:
        raise RuntimeError(
            f"MCP plugin version {selected_version!r} does not match {version!r}: {root}"
        )
    spec = selected_mcp_spec(root, component)
    if version is None and not spec.servers:
        raise RuntimeError("no-MCP validation requires the selected package version")
    if not valid_mcp_surface(root, component, spec):
        raise RuntimeError(f"{component} MCP installation surface is invalid: {root}")
    return root


def mcp_spec_for_plugin(
    plugin: Path, component: str, version: str | None = None
) -> McpSpec:
    """Read a capability only from an identity- and version-checked source package."""
    root = _plugin_root(plugin)
    selected_version = _plugin_version(root, component)
    if version is not None and selected_version != version:
        raise ValueError(
            f"MCP plugin version {selected_version!r} does not match {version!r}: {root}"
        )
    return selected_mcp_spec(root, component)


def component_cache_plugin(home: Path, component: str, version: str) -> Path:
    spec = mcp_spec(component, version)
    _validate_version_component(version)
    return _absolute(home) / CACHE_ROOT / spec.plugin / version


def materialize_component_mcp_cache(
    home: Path,
    component: str,
    version: str,
    *,
    source_plugin: Path | None = None,
) -> Path | None:
    """Repair a cache against the selected source's capability, not the cache's claim."""
    cache_root = _absolute(home) / "plugins" / "cache"
    if not os.path.lexists(cache_root):
        return None
    _require_directory(cache_root, "Codex plugin cache root")
    target = component_cache_plugin(home, component, version)
    if not os.path.lexists(target):
        raise RuntimeError(f"MCP cache plugin is missing: {target}")
    target = _plugin_root(target)
    expected_spec = mcp_spec(component, version)
    if source_plugin is not None:
        source = materialize_component_mcp(source_plugin, component, version)
        expected_spec = mcp_spec_for_plugin(source, component, version)
        sync_mcp_surface(source, target, component, expected_spec)
    if not valid_component_mcp(target, component, version, expected_spec=expected_spec):
        raise RuntimeError(f"{component} MCP cache surface is invalid: {target}")
    return target


def valid_component_mcp(
    plugin: Path,
    component: str,
    version: str | None = None,
    *,
    expected_spec: McpSpec | None = None,
) -> bool:
    try:
        root = _plugin_root(plugin)
        selected_version = _plugin_version(root, component)
        if version is not None and selected_version != version:
            return False
        spec = selected_mcp_spec(root, component)
        if expected_spec is not None and spec != expected_spec:
            return False
        spec = expected_spec or spec
        if version is None and not spec.servers:
            return False
        return valid_mcp_surface(root, component, spec)
    except (OSError, RuntimeError, UnicodeError, ValueError, json.JSONDecodeError):
        return False


def valid_component_mcp_cache(
    home: Path | None,
    component: str,
    version: str,
    *,
    source_plugin: Path | None = None,
    expected_spec: McpSpec | None = None,
) -> bool:
    if home is None:
        return True
    cache_root = _absolute(home) / "plugins" / "cache"
    if not os.path.lexists(cache_root):
        return True
    try:
        _require_directory(cache_root, "Codex plugin cache root")
        if source_plugin is not None:
            expected_spec = mcp_spec_for_plugin(source_plugin, component, version)
        expected_spec = expected_spec or mcp_spec(component, version)
        return valid_component_mcp(
            component_cache_plugin(home, component, version),
            component,
            version,
            expected_spec=expected_spec,
        )
    except (OSError, ValueError, RuntimeError):
        return False


def _plugin_root(plugin: Path) -> Path:
    root = _absolute(plugin)
    _validate_real_path(root, require_exists=True)
    _require_directory(root, "MCP plugin root")
    return root


def _plugin_version(root: Path, component: str) -> str:
    manifest = json.loads(
        (root / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
    )
    if not isinstance(manifest, dict) or (
        manifest.get("name") != mcp_spec(component).plugin
        or manifest.get("repository") != REPOSITORY
    ):
        raise ValueError(f"MCP plugin manifest identity is invalid: {root}")
    value = manifest.get("version")
    if not isinstance(value, str) or not value:
        raise ValueError(f"MCP plugin manifest has no version: {root}")
    return value


def _validate_version_component(version: str) -> None:
    if (
        not isinstance(version, str)
        or not version
        or version in {".", ".."}
        or "/" in version
        or "\\" in version
    ):
        raise ValueError("MCP cache version must be one path component")
