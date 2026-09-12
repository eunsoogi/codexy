"""Shared per-component MCP bootstrap and source/cache consistency checks."""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from pathlib import Path

from .component_mcp_cache import (
    _needs_executable,
    _require_directory,
    _sync_surface,
    _valid_surface_file,
)
from .updater import _absolute, _validate_real_path


RUNTIME_COMMAND = "uv"
REPOSITORY = "https://github.com/eunsoogi/codexy"
BOOTSTRAP_ARGS = (
    "run",
    "--no-project",
    "--script",
    "./mcp/codexy_mcp_bootstrap.py",
)
CACHE_ROOT = Path("plugins/cache/codexy")


@dataclass(frozen=True)
class McpSpec:
    component: str
    plugin: str
    servers: tuple[str, ...]
    surface_paths: tuple[Path, ...]


MCP_SPECS = {
    "core": McpSpec(
        "core",
        "codexy",
        ("watcher",),
        (
            Path(".mcp.json"),
            Path("mcp/codexy_mcp_bootstrap.py"),
            Path("mcp/codexy-mcp-watcher.sh"),
            Path("mcp/codexy-mcp-watcher.cmd"),
        ),
    ),
    "devtools": McpSpec(
        "devtools",
        "codexy-devtools",
        ("lsp", "codegraph"),
        (
            Path(".mcp.json"),
            Path("mcp/codexy_mcp_bootstrap.py"),
            Path("mcp/codexy-mcp-devtools"),
            Path("mcp/codexy-mcp-codegraph"),
            Path("mcp/codexy-mcp-codegraph.cmd"),
            Path("mcp/codexy-mcp-lsp"),
            Path("mcp/codexy-mcp-lsp.cmd"),
            Path("mcp/runtime-platform.sh"),
        ),
    ),
}
MCP_COMPONENTS = frozenset(MCP_SPECS)


def mcp_spec(component: str) -> McpSpec:
    try:
        return MCP_SPECS[component]
    except KeyError as error:
        raise ValueError(
            f"component has no MCP installation contract: {component}"
        ) from error


def mcp_server_config(server: str) -> dict[str, object]:
    return {
        "command": RUNTIME_COMMAND,
        "args": [*BOOTSTRAP_ARGS, server, "--stdio"],
        "cwd": ".",
    }


def mcp_configuration(component: str) -> dict[str, dict[str, object]]:
    return {server: mcp_server_config(server) for server in mcp_spec(component).servers}


def materialize_component_mcp(
    plugin: Path, component: str, version: str | None = None
) -> Path:
    """Validate one plugin's source MCP surface before its host installation."""
    root = _plugin_root(plugin)
    selected_version = _plugin_version(root, component)
    if version is not None and selected_version != version:
        raise RuntimeError(
            f"MCP plugin version {selected_version!r} does not match {version!r}: {root}"
        )
    expected_version = version or selected_version
    if not valid_component_mcp(root, component, expected_version):
        raise RuntimeError(f"{component} MCP installation surface is invalid: {root}")
    return root


def component_cache_plugin(home: Path, component: str, version: str) -> Path:
    spec = mcp_spec(component)
    _validate_version_component(version)
    return _absolute(home) / CACHE_ROOT / spec.plugin / version


def materialize_component_mcp_cache(
    home: Path,
    component: str,
    version: str,
    *,
    source_plugin: Path | None = None,
) -> Path | None:
    """Repair one installed plugin's MCP files in an existing Codex cache."""
    cache_root = _absolute(home) / "plugins" / "cache"
    if not os.path.lexists(cache_root):
        return None
    _require_directory(cache_root, "Codex plugin cache root")
    target = component_cache_plugin(home, component, version)
    if not os.path.lexists(target):
        raise RuntimeError(f"MCP cache plugin is missing: {target}")
    target = _plugin_root(target)
    if source_plugin is not None:
        source = materialize_component_mcp(source_plugin, component, version)
        _sync_surface(source, target, mcp_spec(component).surface_paths)
    if not valid_component_mcp(target, component, version):
        raise RuntimeError(f"{component} MCP cache surface is invalid: {target}")
    return target


def valid_component_mcp(
    plugin: Path, component: str, version: str | None = None
) -> bool:
    try:
        root = _plugin_root(plugin)
        selected_version = _plugin_version(root, component)
        if version is not None and selected_version != version:
            return False
        expected_version = version or selected_version
        configuration = json.loads((root / ".mcp.json").read_text(encoding="utf-8"))
        if configuration != mcp_configuration(component):
            return False
        return all(
            _valid_surface_file(root / path, executable=_needs_executable(path))
            for path in mcp_spec(component).surface_paths
        )
    except (OSError, RuntimeError, UnicodeError, ValueError, json.JSONDecodeError):
        return False


def valid_component_mcp_cache(home: Path | None, component: str, version: str) -> bool:
    if home is None:
        return True
    cache_root = _absolute(home) / "plugins" / "cache"
    if not os.path.lexists(cache_root):
        return True
    try:
        _require_directory(cache_root, "Codex plugin cache root")
        return valid_component_mcp(
            component_cache_plugin(home, component, version), component, version
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
