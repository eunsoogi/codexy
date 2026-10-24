"""Reconcile marker-owned role projections before a lifecycle transaction commits."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from .component_registration_health import synchronize_core_registration
from .component_transition_journal import Journal
from .plugin_resolution import MarketplaceBinding, marketplace_path
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
