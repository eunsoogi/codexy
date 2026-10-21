"""Lifecycle fixtures copy each plugin once and add the bundled Watcher on Windows."""

import json
import os
import shutil
from dataclasses import replace
from pathlib import Path

from codexy_runtime_tools.component_mcp_materialization import (
    NO_MCP_CORE_VERSION,
    mcp_configuration,
)
from codexy_runtime_tools.component_manifest import load_component_manifest
from packages.getcodexy.tests.component_watcher_fixture import install_watcher_runtime

REPOSITORY = Path(__file__).resolve().parents[3]
PLUGIN_NAMES = {
    "core": "codexy",
    "github": "codexy-github",
    "devtools": "codexy-devtools",
}


def configure_plugin_version(plugin: Path, version: str) -> None:
    """Stamp a copied companion plugin with the candidate package version."""
    manifest_path = plugin / ".codex-plugin/plugin.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["version"] = version
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def copy_plugin_package(
    root: Path, plugin_name: str, version: str | None = None
) -> Path:
    """Copy one plugin fixture and make its package declaration match the version."""
    selected_version = version or load_component_manifest().version
    destination = root / "plugins" / plugin_name
    destination.parent.mkdir(parents=True, exist_ok=True)
    plugin = Path(shutil.copytree(REPOSITORY / "plugins" / plugin_name, destination))
    if plugin_name == "codexy":
        configure_core_package(plugin, selected_version)
    else:
        configure_plugin_version(plugin, selected_version)
    return plugin.resolve()


def configure_legacy_core_package(plugin: Path, version: str = "1.13.4") -> None:
    """Make a copied pre-2.0 fixture explicit about its legacy MCP contract."""
    manifest_path = plugin / ".codex-plugin/plugin.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["version"] = version
    manifest["mcpServers"] = "./.mcp.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (plugin / ".mcp.json").write_text(
        json.dumps(mcp_configuration("core", version), indent=2) + "\n",
        encoding="utf-8",
    )


def configure_no_mcp_core_package(
    plugin: Path, version: str = NO_MCP_CORE_VERSION
) -> None:
    """Give a copied candidate the versioned no-MCP contract used by the host."""
    manifest_path = plugin / ".codex-plugin/plugin.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["version"] = version
    manifest.pop("mcpServers", None)
    manifest.pop("mcp_servers", None)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (plugin / ".mcp.json").unlink(missing_ok=True)


def configure_candidate_package(
    plugin: Path, version: str = NO_MCP_CORE_VERSION
) -> None:
    """Keep each copied candidate component on one package release version."""
    if plugin.name == "codexy":
        configure_no_mcp_core_package(plugin, version)
    else:
        configure_plugin_version(plugin, version)


def configure_core_package(plugin: Path, version: str) -> None:
    """Select the fixture contract that the package version declares."""
    if tuple(int(part) for part in version.split(".")) >= (2, 0, 0):
        configure_no_mcp_core_package(plugin, version)
    else:
        configure_legacy_core_package(plugin, version)


def make_no_mcp_core_package(
    plugin: Path, version: str = NO_MCP_CORE_VERSION
) -> tuple[str, object]:
    """Prepare a no-MCP package fixture while retaining its compatibility assets."""
    configure_no_mcp_core_package(plugin, version)
    for relative in (
        "mcp/codexy_mcp_bootstrap.py",
        "mcp/codexy-mcp-watcher.sh",
        "mcp/codexy-mcp-watcher.cmd",
    ):
        if not (plugin / relative).is_file():
            raise AssertionError(f"compatibility asset is missing: {relative}")
    base = load_component_manifest()
    manifest = replace(
        base,
        version=version,
        components=tuple(
            replace(component, version=version) for component in base.components
        ),
    )
    return version, manifest


def materialize_plugins(
    state,
    *components: str,
    plugin_names: dict[str, str],
    version: str,
) -> None:
    """Stage selected lifecycle fixtures without launching their plugin assets."""
    for component in components:
        plugin_name = plugin_names[component]
        plugin = state.marketplace / "plugins" / plugin_name
        if not plugin.exists():
            plugin = copy_plugin_package(state.marketplace, plugin_name, version)
        elif component == "core":
            configure_core_package(plugin, version)
        else:
            continue
        if component == "core":
            install_watcher_runtime(plugin)


def populate_plugins(marketplace: Path) -> None:
    for plugin in ("codexy", "codexy-github", "codexy-devtools"):
        destination = marketplace / "plugins" / plugin
        created = not destination.exists()
        if created:
            shutil.copytree(REPOSITORY / "plugins" / plugin, destination)
            if plugin == "codexy":
                configure_core_package(destination, "1.13.4")
        if created and plugin == "codexy" and os.name == "nt":
            install_watcher_runtime(destination)
