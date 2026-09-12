"""MCP source and cache materialization for component lifecycle operations."""

from __future__ import annotations

from pathlib import Path

from .component_manifest import ComponentManifest
from .component_mcp_materialization import (
    MCP_COMPONENTS,
    materialize_component_mcp,
    materialize_component_mcp_cache,
)
from .plugin_resolution import MarketplaceBinding, marketplace_path


def materialize_mcp_sources(
    root: MarketplaceBinding,
    manifest: ComponentManifest,
    components: tuple[str, ...],
) -> None:
    marketplace = marketplace_path(root)
    for component in manifest.component_ids:
        if component not in components or component not in MCP_COMPONENTS:
            continue
        plugin = marketplace / "plugins" / manifest.component(component).plugin
        materialize_component_mcp(plugin, component, manifest.version)


def materialize_mcp_caches(
    home: Path,
    root: MarketplaceBinding,
    manifest: ComponentManifest,
    components: tuple[str, ...],
) -> None:
    marketplace = marketplace_path(root)
    for component in manifest.component_ids:
        if component not in components or component not in MCP_COMPONENTS:
            continue
        source = marketplace / "plugins" / manifest.component(component).plugin
        materialize_component_mcp_cache(
            home,
            component,
            manifest.version,
            source_plugin=source,
        )
