"""Canonical ordinary-file registration checks used by the doctor report."""

from __future__ import annotations

import json
from pathlib import Path

from .component_core_hooks import (
    DEPENDENCIES as CORE_HOOK_DEPENDENCIES,
    LAUNCHERS as CORE_HOOK_LAUNCHERS,
)
from .component_hook_registration import HOOKS, LAUNCHERS
from .component_mcp_materialization import (
    MCP_COMPONENTS,
    McpSpec,
    mcp_configuration,
    mcp_spec_for_plugin,
    valid_component_mcp,
)
from .component_manifest import load_component_manifest
from .component_registration_catalog import CATALOGS, _catalog_agent_files
from .component_registration_files import _launcher, _regular, _skill, _text
from .component_transition_journal import Journal
from .plugin_resolution import MarketplaceBinding, marketplace_path
from .updater import sync_agents

MANAGED_MARKERS = {
    "core": "# CODEXY MANAGED AGENT\n",
    "github": "# Managed by Codexy GitHub.\n",
}
MANAGED_ROOTS = {
    "core": Path("agents/codexy"),
    "github": Path("agents/codexy-github"),
}
REGISTRATION_SOURCE = "codex-home-standalone-agent-projection"
REGISTRATION_SCOPE = "current-doctor-invocation"
REGISTRATION_REPAIR = "repair the Codexy registration, then rerun getcodexy doctor"


def synchronize_core_registration(
    home: Path, root: MarketplaceBinding, journal: Journal
) -> None:
    """Reconcile core roles after install-like operations; failures trigger rollback."""
    if (
        journal.command not in {"install", "update", "bootstrap"}
        or "core" not in journal.target
    ):
        return
    result = sync_agents(marketplace_path(root) / "plugins" / "codexy", home, "install")
    if result.status == "completed":
        return
    diagnostics = "; ".join(result.diagnostics)
    detail = f": {diagnostics}" if diagnostics else ""
    raise RuntimeError(f"Codexy role registration failed ({result.status}){detail}")


def _quoted_end(text: str, index: int) -> int | None:
    """Find the quoted value's end while honoring escapes in basic strings."""
    quote, index = text[index], index + 1
    while index < len(text):
        if quote == '"' and text[index] == "\\":
            index += 2
        elif text[index] == quote:
            return index + 1
        else:
            index += 1
    return None


MCP = mcp_configuration("devtools")
CORE_MCP = mcp_configuration("core")


def valid_registration(
    plugin: Path,
    component: str,
    version: str | None = None,
    *,
    expected_mcp_spec: McpSpec | None = None,
) -> bool:
    """Require exactly the packaged registration and its local launch targets."""
    try:
        # Install flows pass their validated release; doctor uses its bundled manifest.
        selected_version = (
            load_component_manifest().version if version is None else version
        )
        package_mcp_spec = None
        if component in MCP_COMPONENTS:
            package_mcp_spec = mcp_spec_for_plugin(plugin, component, selected_version)
            if expected_mcp_spec is not None and package_mcp_spec != expected_mcp_spec:
                return False
            package_mcp_spec = expected_mcp_spec or package_mcp_spec
        if component == "devtools":
            return valid_component_mcp(
                plugin,
                component,
                selected_version,
                expected_spec=package_mcp_spec,
            )
        core_mcp = component == "core" and valid_component_mcp(
            plugin,
            component,
            selected_version,
            expected_spec=package_mcp_spec,
        )
        return (
            (component != "core" or core_mcp)
            and _text(plugin / "agents/catalog.toml", plugin) == CATALOGS[component]
            and json.loads(_text(plugin / "hooks/hooks.json", plugin))
            == HOOKS[component]
            and all(
                _regular(plugin / f"agents/{name}", plugin, 'model = "')
                for name in _catalog_agent_files(
                    _text(plugin / "agents/catalog.toml", plugin)
                )
            )
            and all(_launcher(plugin / path, plugin) for path in LAUNCHERS[component])
            and _skill(plugin, component)
        )
    except (KeyError, OSError, UnicodeDecodeError, SyntaxError, ValueError):
        return False


def registration_role(
    name: str, state: str, cause: str | None = None, conflict: bool = False
) -> dict[str, object]:
    """Prioritize unmanaged-file conflicts when selecting role recovery guidance."""
    recovery = (
        "move or remove the unmanaged role file, then rerun getcodexy doctor"
        if conflict
        else REGISTRATION_REPAIR
        if cause
        else None
    )
    return {"file": name, "state": state, "cause": cause, "recovery": recovery}


def _registered_role(
    home: Path,
    root: Path,
    name: str,
    expected: bytes,
    marker: str,
    read_regular,
    max_bytes: int,
) -> dict[str, object]:
    try:  # Inspect the marker-owned role file as a bounded regular input.
        current = (
            read_regular(home, root.relative_to(home) / name, max_bytes)
            .decode()
            .replace("\r\n", "\n")
            .replace("\r", "\n")
        )
    except FileNotFoundError:
        return registration_role(name, "missing", "managed role file is missing")
    except (OSError, UnicodeDecodeError, ValueError):
        return registration_role(
            name,
            "unmanaged-conflict",
            "managed role file is unreadable or not a regular file",
            True,
        )
    if not current.startswith(marker):
        return registration_role(
            name,
            "unmanaged-conflict",
            "existing role file is not marker-owned by Codexy",
            True,
        )
    exact = current.encode() == expected
    return registration_role(
        name,
        "exact" if exact else "stale",
        None if exact else "registered role differs from the package",
    )


def registration_report(
    component: str,
    observed: bool,
    roles: list[dict[str, object]],
    managed: tuple[str, ...],
    unmanaged: tuple[str, ...],
    expected_count: int,
    state: str,
    error: str | None = None,
) -> dict[str, object]:
    """Prefer role defects, then unmanaged extras, over component errors."""
    problem = next((item for item in roles if item["state"] != "exact"), None)
    cause = problem["cause"] if problem else error
    recovery = (
        problem["recovery"] if problem else REGISTRATION_REPAIR if cause else None
    )
    if not problem and unmanaged:
        cause = f"unmanaged role file is present: {unmanaged[0]}"
        recovery = "move or remove the unmanaged role file, then rerun getcodexy doctor"
    return {
        "component": component,
        "source": REGISTRATION_SOURCE,
        "scope": REGISTRATION_SCOPE,
        "observed": observed,
        "state": state,
        "roles": roles,
        "managed_extra": list(managed),
        "unmanaged_extra": list(unmanaged),
        "expected_count": expected_count,
        "exact_count": sum(item["state"] == "exact" for item in roles),
        "cause": cause,
        "recovery": recovery,
    }
