"""Public, dependency-aware activation for Codexy's GitHub component."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from .component_integrity import frozen_component
from .activation_transaction import ActivationSnapshot
from .plugin_resolution import (
    official_named_install,
    preflight_named_install,
)
from .pre_session import _json, _run, official_marketplace_root
from .updater import SyncResult, _absolute, _validate_real_path, sync_agents
from .version_lock import default_package_version


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]
CoreSynchronizer = Callable[[Path, Path, str], SyncResult]
GithubActivator = Callable[[Path, Path], bool]


@dataclass(frozen=True)
class GithubPreSessionResult:
    core_root: Path
    github_root: Path
    version: str
    changed: bool


def run_github_pre_session(
    codex_home: str | os.PathLike[str],
    *,
    codex: Path,
    runner: Runner | None = None,
    synchronize: CoreSynchronizer = sync_agents,
    activate_github: GithubActivator | None = None,
    package_version: str | None = None,
) -> GithubPreSessionResult:
    home = _absolute(codex_home)
    _validate_real_path(home, require_exists=False)
    executable = trusted_codex(codex)
    invoke = runner or (lambda command: _run(command, home))
    release = package_version or default_package_version()
    marketplace_root = official_marketplace_root(executable, invoke, release)
    before = _json(invoke([str(executable), "plugin", "list", "--json"]), "plugin list")
    for name in ("codexy", "codexy-github"):
        preflight_named_install(before, marketplace_root, name)
        reject_disabled(before, name)
    added = [
        identity
        for identity, name in (
            ("codexy@codexy", "codexy"),
            ("codexy-github@codexy", "codexy-github"),
        )
        if not enabled(before, name)
    ]
    snapshot = ActivationSnapshot.capture(home)
    try:
        for identity in ("codexy@codexy", "codexy-github@codexy"):
            _json(
                invoke([str(executable), "plugin", "add", identity, "--json"]),
                "plugin add",
            )
        installed = _json(
            invoke([str(executable), "plugin", "list", "--json"]), "plugin list"
        )
        core_root, core_version = official_named_install(
            installed, marketplace_root, release, "codexy"
        )
        github_root, github_version = official_named_install(
            installed, marketplace_root, release, "codexy-github"
        )
        if core_version != github_version:
            raise ValueError("Codexy core and GitHub plugin versions must match")
        with (
            frozen_component(core_root, "codexy", core_version) as trusted_core,
            frozen_component(
                github_root, "codexy-github", github_version
            ) as trusted_github,
        ):
            core_changed = activate_core(trusted_core, home, synchronize)
            activate = activate_github or sync_github_agent
            github_changed = activate(trusted_github, home)
    except Exception as error:
        failures = []
        try:
            snapshot.restore()
        except Exception:
            failures.append("agent projections")
        failures.extend(rollback_install(executable, invoke, added))
        if failures:
            joined = ", ".join(failures)
            raise RuntimeError(
                f"GitHub activation failed; rollback also failed: {joined}"
            ) from error
        raise
    return GithubPreSessionResult(
        core_root, github_root, core_version, core_changed or github_changed
    )


def trusted_codex(path: Path) -> Path:
    if not path.is_absolute():
        raise ValueError("host-provided Codex executable must be an absolute path")
    executable = _absolute(path)
    _validate_real_path(executable, require_exists=True)
    if not executable.is_file():
        raise ValueError("host-provided Codex executable must be a regular file")
    return executable


def enabled(payload: object, name: str) -> bool:
    if not isinstance(payload, dict) or not isinstance(payload.get("installed"), list):
        return False
    return any(
        isinstance(item, dict)
        and item.get("pluginId") == f"{name}@codexy"
        and item.get("enabled") is True
        for item in payload["installed"]
    )


def reject_disabled(payload: object, name: str) -> None:
    if not isinstance(payload, dict) or not isinstance(payload.get("installed"), list):
        return
    if any(
        isinstance(item, dict)
        and item.get("pluginId") == f"{name}@codexy"
        and item.get("installed") is True
        and item.get("enabled") is False
        for item in payload["installed"]
    ):
        raise ValueError(f"refusing to change a disabled {name} install")


def rollback_install(
    executable: Path, invoke: Runner, identities: list[str]
) -> list[str]:
    failures = []
    for identity in reversed(identities):
        try:
            _json(
                invoke([str(executable), "plugin", "remove", identity, "--json"]),
                "plugin remove",
            )
        except Exception:
            failures.append(identity)
    return failures


def activate_core(root: Path, home: Path, synchronize: CoreSynchronizer) -> bool:
    current = synchronize(root, home, "check")
    if current.status == "ready":
        return False
    if current.status != "update_required":
        raise RuntimeError(f"core agent projection check failed: {current.status}")
    applied = synchronize(root, home, "install")
    if applied.status != "completed":
        raise RuntimeError(f"core agent projection install failed: {applied.status}")
    return applied.changed


def sync_github_agent(root: Path, home: Path) -> bool:
    root = _absolute(root)
    script = root / "skills/git-workflow/scripts/bootstrap_codexy_github_agent.py"
    _validate_real_path(script, require_exists=True)
    environment = os.environ.copy()
    environment.pop("PYTHONHOME", None)
    environment.pop("PYTHONPATH", None)
    environment["PYTHONNOUSERSITE"] = "1"
    command = [sys.executable, "-B", str(script), "--codex-home", str(home)]
    current = subprocess.run(
        command + ["--diagnose"],
        text=True,
        capture_output=True,
        check=False,
        env=environment,
    )
    if current.returncode == 0 and "D role-discovery: PASS" in current.stdout:
        return False
    applied = subprocess.run(
        command, text=True, capture_output=True, check=False, env=environment
    )
    if applied.returncode or "D bootstrap: RESTART_REQUIRED" not in applied.stdout:
        raise RuntimeError("GitHub specialist activation failed")
    verified = subprocess.run(
        command + ["--diagnose"],
        text=True,
        capture_output=True,
        check=False,
        env=environment,
    )
    if verified.returncode or "D role-discovery: PASS" not in verified.stdout:
        raise RuntimeError("GitHub specialist activation was not verified")
    return True


def main() -> int:
    parser = argparse.ArgumentParser(prog="codexy-github-install", allow_abbrev=False)
    parser.add_argument(
        "--codex",
        type=Path,
        required=True,
        help="absolute path supplied by the trusted Codex host",
    )
    parser.add_argument(
        "--codex-home",
        type=Path,
        default=Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")),
    )
    arguments = parser.parse_args()
    try:
        result = run_github_pre_session(arguments.codex_home, codex=arguments.codex)
    except Exception as error:
        print(f"codexy GitHub install: {error}", file=sys.stderr)
        return 1
    status = "changed" if result.changed else "current"
    print(f"codexy-github {result.version} {status}; start a fresh Codex task")
    return 0
