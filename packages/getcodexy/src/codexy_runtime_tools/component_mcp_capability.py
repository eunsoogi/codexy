"""Select and validate the MCP contract declared by a plugin package."""

from __future__ import annotations

import json
import os
import stat
from dataclasses import dataclass
from pathlib import Path

from .component_mcp_cache import (
    _needs_executable,
    _require_directory,
    _sync_surface,
    _valid_surface_file,
)
from .updater import _absolute, _validate_real_path


CAPABILITY_PATH = Path(".codex-plugin/mcp-capability.json")
NO_MCP_CAPABILITY = {
    "schema": "codexy.mcp-capability.v1",
    "component": "core",
    "servers": [],
}


@dataclass(frozen=True)
class McpSpec:
    component: str
    plugin: str
    servers: tuple[str, ...]
    surface_paths: tuple[Path, ...]


MCP_SPECS = {
    # Read-only compatibility for existing registrations; current launchers reject Watcher.
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
NO_MCP_CORE_SPEC = McpSpec("core", "codexy", (), (CAPABILITY_PATH,))


def mcp_spec(component: str, version: str | None = None) -> McpSpec:
    """Return the declared-registration layout; versions do not select absence."""
    del version
    try:
        return MCP_SPECS[component]
    except KeyError as error:
        raise ValueError(
            f"component has no MCP installation contract: {component}"
        ) from error


def mcp_configuration(component: str, version: str | None = None):
    """Build a strict legacy registration fixture for the named component."""
    return {
        server: {
            "command": "uv",
            "args": [
                "run",
                "--no-project",
                "--script",
                "./mcp/codexy_mcp_bootstrap.py",
                server,
                "--stdio",
            ],
            "cwd": ".",
        }
        for server in mcp_spec(component, version).servers
    }


def selected_mcp_spec(root: Path, component: str) -> McpSpec:
    """Use an explicit package contract for no-MCP; otherwise require legacy registration."""
    legacy = mcp_spec(component)
    capability = root / CAPABILITY_PATH
    if component != "core":
        if os.path.lexists(capability):
            raise ValueError("no-MCP capability is valid only for the core plugin")
        return legacy
    if not os.path.lexists(capability):
        return legacy
    if not _valid_no_mcp_capability(root):
        raise ValueError("core MCP capability contract is invalid")
    return NO_MCP_CORE_SPEC


def valid_mcp_surface(root: Path, component: str, spec: McpSpec) -> bool:
    """Require exact agreement between a selected contract and plugin files."""
    if component == "core" and spec == NO_MCP_CORE_SPEC:
        return _valid_no_mcp_capability(root) and _valid_absent_surface(root)
    if spec != mcp_spec(component) or os.path.lexists(root / CAPABILITY_PATH):
        return False
    try:
        manifest = json.loads(
            (root / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
        )
        if (
            not isinstance(manifest, dict)
            or manifest.get("mcpServers") != "./.mcp.json"
            or "mcp_servers" in manifest
        ):
            return False
        configuration = json.loads((root / ".mcp.json").read_text(encoding="utf-8"))
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError):
        return False
    if configuration != mcp_configuration(component):
        return False
    return all(
        _valid_surface_file(root / path, executable=_needs_executable(path))
        for path in spec.surface_paths
    )


def sync_mcp_surface(source: Path, target: Path, component: str, spec: McpSpec) -> None:
    """Copy the selected contract and remove stale registration metadata from cache."""
    paths = list(spec.surface_paths)
    if component == "core":
        # The manifest controls host discovery, so same-version cache repair must sync it too.
        paths.insert(0, Path(".codex-plugin/plugin.json"))
    _sync_surface(source, target, paths)
    if component == "core":
        obsolete = Path(".mcp.json") if not spec.servers else CAPABILITY_PATH
        _remove_optional_regular_file(target, obsolete)
    elif os.path.lexists(target / CAPABILITY_PATH):
        _remove_optional_regular_file(target, CAPABILITY_PATH)


def _valid_no_mcp_capability(root: Path) -> bool:
    path = root / CAPABILITY_PATH
    if not _valid_surface_file(path, executable=False):
        return False
    try:
        capability = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object
        )
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError):
        return False
    return capability == NO_MCP_CAPABILITY


def _valid_absent_surface(root: Path) -> bool:
    """Reject host registration metadata while keeping dormant compatibility assets."""
    try:
        manifest = json.loads(
            (root / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
        )
        if (
            not isinstance(manifest, dict)
            or "mcpServers" in manifest
            or "mcp_servers" in manifest
        ):
            return False
        config = root / ".mcp.json"
        _validate_real_path(_absolute(config), require_exists=False)
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError):
        return False
    return not os.path.lexists(config)


def _remove_optional_regular_file(root: Path, relative: Path) -> None:
    """Remove only a plain stale cache file; links and special files fail closed."""
    target = root / relative
    _validate_real_path(_absolute(target), require_exists=False)
    if not os.path.lexists(target):
        return
    metadata = target.lstat()
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise RuntimeError(f"MCP cache file is not a regular file: {target}")
    target.unlink()


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate MCP capability key: {key}")
        result[key] = value
    return result
