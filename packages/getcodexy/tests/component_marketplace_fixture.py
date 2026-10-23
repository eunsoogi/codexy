"""Marketplace fixtures isolate legacy Watcher binaries to upgrade coverage."""

import json
import os
import shutil
from dataclasses import replace
from pathlib import Path

from codexy_runtime_tools.component_mcp_capability import (
    NO_MCP_CAPABILITY,
    mcp_configuration,
)
from codexy_runtime_tools.component_manifest import load_component_manifest
from packages.getcodexy.tests.component_watcher_fixture import (
    LEGACY_VERSION,
    install_watcher_runtime,
)

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
    """Build a synthetic registered-core package for legacy validation coverage."""
    manifest_path = plugin / ".codex-plugin/plugin.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["version"] = version
    manifest["mcpServers"] = "./.mcp.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (plugin / ".codex-plugin/mcp-capability.json").unlink(missing_ok=True)
    (plugin / ".mcp.json").write_text(
        json.dumps(mcp_configuration("core", version), indent=2) + "\n",
        encoding="utf-8",
    )
    mcp = plugin / "mcp"
    mcp.mkdir(parents=True, exist_ok=True)
    launcher = mcp / "codexy-mcp-watcher.sh"
    launcher.write_text("#!/bin/sh\n", encoding="utf-8")
    launcher.chmod(0o755)
    (mcp / "codexy_mcp_bootstrap.py").write_text(
        "#!/usr/bin/env python3\n", encoding="utf-8"
    )
    (mcp / "codexy-mcp-watcher.cmd").write_text("@echo off\n", encoding="utf-8")


def configure_no_mcp_core_package(plugin: Path, version: str | None = None) -> None:
    """Give a copied candidate the explicit no-MCP package contract."""
    selected_version = load_component_manifest().version if version is None else version
    manifest_path = plugin / ".codex-plugin/plugin.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["version"] = selected_version
    manifest.pop("mcpServers", None)
    manifest.pop("mcp_servers", None)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (plugin / ".mcp.json").unlink(missing_ok=True)
    capability_path = plugin / ".codex-plugin/mcp-capability.json"
    capability_path.write_text(
        json.dumps(NO_MCP_CAPABILITY, indent=2) + "\n", encoding="utf-8"
    )
    # Current source fixtures must not carry retired launchers from older inputs.
    mcp = plugin / "mcp"
    for name in (
        "codexy_mcp_bootstrap.py",
        "codexy-mcp-watcher.sh",
        "codexy-mcp-watcher.cmd",
    ):
        (mcp / name).unlink(missing_ok=True)
    if mcp.is_dir() and not any(mcp.iterdir()):
        mcp.rmdir()


def configure_candidate_package(plugin: Path, version: str | None = None) -> None:
    """Keep each copied candidate component on one package release version."""
    selected_version = load_component_manifest().version if version is None else version
    if plugin.name == "codexy":
        configure_no_mcp_core_package(plugin, selected_version)
    else:
        configure_plugin_version(plugin, selected_version)


def configure_core_package(plugin: Path, version: str) -> None:
    """Build legacy or no-MCP test fixtures with explicit package metadata."""
    if tuple(int(part) for part in version.split(".")) >= (2, 0, 0):
        configure_no_mcp_core_package(plugin, version)
    else:
        configure_legacy_core_package(plugin, version)


def make_no_mcp_core_package(
    plugin: Path, version: str | None = None
) -> tuple[str, object]:
    """Prepare a no-MCP package fixture without retired Watcher entrypoints."""
    selected_version = load_component_manifest().version if version is None else version
    configure_no_mcp_core_package(plugin, selected_version)
    for relative in (
        "mcp/codexy_mcp_bootstrap.py",
        "mcp/codexy-mcp-watcher.sh",
        "mcp/codexy-mcp-watcher.cmd",
    ):
        if (plugin / relative).exists():
            raise AssertionError(f"retired entrypoint is present: {relative}")
    base = load_component_manifest()
    manifest = replace(
        base,
        version=selected_version,
        components=tuple(
            replace(component, version=selected_version)
            for component in base.components
        ),
    )
    return selected_version, manifest


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


def populate_plugins(
    marketplace: Path, *, verified_legacy_upgrade_fixture: bool = False
) -> None:
    # Checkout and public 1.13.4 share a version string; only an explicit
    # upgrade fixture may add the verified native Watcher binary.
    for plugin in ("codexy", "codexy-github", "codexy-devtools"):
        destination = marketplace / "plugins" / plugin
        created = not destination.exists()
        if created:
            shutil.copytree(REPOSITORY / "plugins" / plugin, destination)
            if plugin == "codexy":
                configure_core_package(destination, LEGACY_VERSION)
        if (
            created
            and plugin == "codexy"
            and verified_legacy_upgrade_fixture
            and os.name == "nt"
        ):
            install_watcher_runtime(destination, verified_legacy_upgrade_fixture=True)
