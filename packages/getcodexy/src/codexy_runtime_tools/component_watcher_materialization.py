"""Compatibility names for the shared component MCP installation contract."""

from __future__ import annotations

from pathlib import Path

from .component_mcp_materialization import (
    CACHE_ROOT,
    component_cache_plugin,
    materialize_component_mcp,
    materialize_component_mcp_cache,
    valid_component_mcp,
    valid_component_mcp_cache,
)


# Kept for existing callers. The registered MCP command now uses the common
# metadata-driven bootstrap, so these are compatibility-launcher paths, not
# generated registration targets.
WATCHER_COMMAND = Path("mcp/codexy-mcp-watcher")
WATCHER_SOURCE = Path("mcp/codexy-mcp-watcher.sh")
WATCHER_WINDOWS = Path("mcp/codexy-mcp-watcher.exe")
WATCHER_RUNTIME = Path("runtime/codexy-mcp-watcher-windows-x86_64.exe")
WATCHER_CACHE_ROOT = CACHE_ROOT / "codexy"


def watcher_entrypoint(plugin: Path) -> Path:
    """Return the retained source launcher for legacy callers."""
    return plugin / WATCHER_SOURCE


def materialize_watcher(plugin: Path, home: Path | None = None) -> Path:
    del home
    return materialize_component_mcp(plugin, "core")


def valid_watcher_entrypoint(plugin: Path) -> bool:
    return valid_component_mcp(plugin, "core")


def watcher_cache_plugin(home: Path, version: str) -> Path:
    return component_cache_plugin(home, "core", version)


def materialize_watcher_cache(home: Path, version: str) -> Path | None:
    return materialize_component_mcp_cache(home, "core", version)


def valid_watcher_cache(home: Path | None, version: str) -> bool:
    return valid_component_mcp_cache(home, "core", version)
