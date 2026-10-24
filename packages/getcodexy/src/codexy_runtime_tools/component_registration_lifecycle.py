"""Reconcile marker-owned role projections before a lifecycle transaction commits."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from .component_registration_health import synchronize_core_registration
from .component_registration_projection import (
    MANAGED_ROOTS,
    has_managed_role_marker,
    managed_role_files,
)
from .component_transition_journal import Journal
from .plugin_resolution import MarketplaceBinding, marketplace_path
from .component_transaction_snapshot import _read_regular, _unlink_regular
from .updater import _validate_real_path, sync_agents


def synchronize_component_registrations(
    home: Path, root: MarketplaceBinding, journal: Journal
) -> None:
    """Use the existing installers and uninstallers inside the durable transaction."""
    synchronize_core_registration(home, root, journal)
    if journal.command != "remove":
        return
    removed = set(journal.before) - set(journal.target)
    if "core" in removed:
        _uninstall_core(home, root)
    if "github" in removed:
        _uninstall_github(home, root)
    for component in MANAGED_ROOTS:
        if component in removed:
            _remove_remaining_managed_roles(home, component)


def _uninstall_core(home: Path, root: MarketplaceBinding) -> None:
    result = sync_agents(
        marketplace_path(root) / "plugins" / "codexy", home, "uninstall"
    )
    if result.status != "completed":
        diagnostics = "; ".join(result.diagnostics)
        detail = f": {diagnostics}" if diagnostics else ""
        raise RuntimeError(f"Codexy role removal failed ({result.status}){detail}")


def _uninstall_github(home: Path, root: MarketplaceBinding) -> None:
    script = (
        marketplace_path(root)
        / "plugins/codexy-github/skills/git-workflow/scripts/bootstrap_codexy_github_agent.py"
    )
    _validate_real_path(script, require_exists=True)
    environment = os.environ.copy()
    # Do not inherit caller import-path overrides into the bootstrap subprocess.
    _ = environment.pop("PYTHONHOME", None)
    _ = environment.pop("PYTHONPATH", None)
    environment["PYTHONNOUSERSITE"] = "1"
    command = [
        sys.executable,
        "-B",
        str(script),
        "--codex-home",
        str(home),
        "--uninstall",
    ]
    result = subprocess.run(
        command,
        capture_output=True,
        check=False,
        env=environment,
        text=True,
    )
    if (
        result.returncode != 0
        or "D bootstrap: REMOVED" not in result.stdout.splitlines()
    ):
        diagnostics = "; ".join(filter(str.strip, (result.stdout, result.stderr)))
        detail = f": {diagnostics}" if diagnostics else ""
        raise RuntimeError(f"Codexy GitHub role removal failed{detail}")


def _remove_remaining_managed_roles(home: Path, component: str) -> None:
    """Clear stale marker-owned roles omitted by the current package catalog."""
    root = home / MANAGED_ROOTS[component]
    # Current uninstallers know the package catalog; legacy marked files still
    # belong to this component and must be removed within this transaction.
    for name in managed_role_files(home, component):
        path = root / name
        try:
            metadata = path.lstat()
            identity = (metadata.st_dev, metadata.st_ino)
            contents = _read_regular(path)
        except FileNotFoundError:
            continue
        if contents is None or not has_managed_role_marker(contents, component):
            continue
        current = path.lstat()
        if (current.st_dev, current.st_ino) != identity:
            continue
        _unlink_regular(path, identity)
    remaining = managed_role_files(home, component)
    if remaining:
        raise RuntimeError(
            f"managed {component} roles remain after removal: {', '.join(remaining)}"
        )
