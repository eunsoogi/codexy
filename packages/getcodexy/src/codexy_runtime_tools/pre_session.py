from __future__ import annotations

import json
import os
import shutil
import subprocess
from dataclasses import dataclass
from importlib.metadata import version as distribution_version
from pathlib import Path
from typing import Callable

from .plugin_resolution import (
    named_marketplace as _named_marketplace,
    official_install as _official_install,
    official_marketplace as _official_marketplace,
    preflight_install as _preflight,
)
from .updater import SyncResult, _absolute, _validate_real_path, sync_agents
Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]


@dataclass(frozen=True)
class PreSessionResult:
    plugin_root: Path
    version: str
    changed: bool


def run_pre_session(
    codex_home: str | os.PathLike[str],
    *,
    codex: Path | None = None,
    runner: Runner | None = None,
    synchronize: Callable[[Path, Path, str], SyncResult] = sync_agents,
    package_version: str | None = None,
) -> PreSessionResult:
    home = _absolute(codex_home)
    executable = codex or _find_codex()
    invoke = runner or (lambda command: _run(command, home))
    _validate_real_path(home, require_exists=False)

    market = _json(
        invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
        "marketplace list",
    )
    if not _named_marketplace(market):
        _json(
            invoke(
                [
                    str(executable),
                    "plugin",
                    "marketplace",
                    "add",
                    "eunsoogi/codexy",
                    "--ref",
                    "main",
                    "--json",
                ]
            ),
            "marketplace add",
        )
        market = _json(
            invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
            "marketplace list",
        )
    marketplace_root = _official_marketplace(market)

    before = _json(
        invoke([str(executable), "plugin", "list", "--json"]),
        "plugin list",
    )
    _preflight(before, marketplace_root)

    _json(
        invoke(
            [str(executable), "plugin", "marketplace", "upgrade", "codexy", "--json"]
        ),
        "marketplace upgrade",
    )
    market = _json(
        invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
        "marketplace list",
    )
    marketplace_root = _official_marketplace(market)
    _json(
        invoke([str(executable), "plugin", "add", "codexy@codexy", "--json"]),
        "plugin add",
    )
    plugin, version = _official_install(
        _json(invoke([str(executable), "plugin", "list", "--json"]), "plugin list"),
        marketplace_root,
        package_version or distribution_version("getcodexy"),
    )
    current = synchronize(plugin, home, "check")
    if current.status == "ready":
        return PreSessionResult(plugin, version, False)
    if current.status != "update_required":
        raise RuntimeError(f"agent projection check failed: {current.status}")

    applied = synchronize(plugin, home, "install")
    if applied.status != "completed":
        raise RuntimeError(f"agent projection install failed: {applied.status}")
    return PreSessionResult(plugin, version, applied.changed)


def _find_codex() -> Path:
    candidate = shutil.which("codex")
    if not candidate:
        raise RuntimeError("official Codex CLI is not on PATH")
    path = Path(candidate).resolve(strict=True)
    if not path.is_absolute() or not path.is_file():
        raise RuntimeError("official Codex CLI must resolve to an absolute regular file")
    return path


def _run(
    command: list[str],
    codex_home: Path,
) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    for name in (
        "GIT_DIR",
        "GIT_EXEC_PATH",
        "GIT_SSH",
        "GIT_SSH_COMMAND",
        "GIT_WORK_TREE",
        "SSH_ASKPASS",
        "PYTHONHOME",
        "PYTHONPATH",
    ):
        environment.pop(name, None)
    environment.update(
        {
            "CODEX_HOME": str(codex_home),
            "GIT_CONFIG_COUNT": "0",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_TERMINAL_PROMPT": "0",
        }
    )
    return subprocess.run(
        command,
        text=True,
        capture_output=True,
        check=False,
        env=environment,
    )


def _json(done: subprocess.CompletedProcess[str], stage: str) -> object:
    if done.returncode:
        raise RuntimeError(f"{stage} failed")
    try:
        return json.loads(done.stdout)
    except json.JSONDecodeError as error:
        raise ValueError(f"{stage} returned invalid JSON") from error
