from __future__ import annotations

import json
import os
import shutil
import subprocess
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from .plugin_resolution import (
    named_marketplace as _named_marketplace,
    official_install as _official_install,
    official_marketplace as _official_marketplace,
    preflight_install as _preflight,
)
from .updater import SyncResult, _absolute, _validate_real_path, sync_agents
from .version_lock import default_package_version

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

    target_version = package_version or default_package_version()
    market = _json(
        invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
        "marketplace list",
    )
    marketplace_root = (
        _official_marketplace(market) if _named_marketplace(market) else None
    )
    existing_marketplace = marketplace_root is not None
    if existing_marketplace:
        before = _json(
            invoke([str(executable), "plugin", "list", "--json"]),
            "plugin list",
        )
        _preflight(before, marketplace_root)
    marketplace_root = reconcile_official_marketplace_root(
        executable, invoke, target_version, home, market
    )
    if not existing_marketplace:
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
    marketplace_root = _official_marketplace(
        _json(
            invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
            "marketplace list",
        )
    )
    _json(
        invoke([str(executable), "plugin", "add", "codexy@codexy", "--json"]),
        "plugin add",
    )
    plugin, version = _official_install(
        _json(invoke([str(executable), "plugin", "list", "--json"]), "plugin list"),
        marketplace_root,
        target_version,
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


def official_marketplace_root(
    executable: Path, invoke: Runner, target_version: str | None = None
) -> Path:
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
                    f"v{target_version or default_package_version()}",
                    "--json",
                ]
            ),
            "marketplace add",
        )
        market = _json(
            invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
            "marketplace list",
        )
    return _official_marketplace(market)


def reconcile_official_marketplace_root(
    executable: Path,
    invoke: Runner,
    target_version: str,
    home: Path,
    market: object | None = None,
) -> Path:
    if market is None:
        market = _json(
            invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
            "marketplace list",
        )
    if _named_marketplace(market):
        _official_marketplace(market)
        previous_ref, config_snapshot = _marketplace_ref(home)
        _json(
            invoke(
                [str(executable), "plugin", "marketplace", "remove", "codexy", "--json"]
            ),
            "marketplace remove",
        )
        try:
            _add_marketplace(executable, invoke, f"v{target_version}")
            market = _json(
                invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
                "marketplace list",
            )
            return _official_marketplace(market)
        except Exception:
            try:
                _add_marketplace(executable, invoke, previous_ref)
            finally:
                _restore_config(home, config_snapshot)
            raise
    else:
        _add_marketplace(executable, invoke, f"v{target_version}")
    market = _json(
        invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
        "marketplace list",
    )
    return _official_marketplace(market)


def _add_marketplace(executable: Path, invoke: Runner, ref: str) -> None:
    _json(
        invoke(
            [
                str(executable),
                "plugin",
                "marketplace",
                "add",
                "eunsoogi/codexy",
                "--ref",
                ref,
                "--json",
            ]
        ),
        "marketplace add",
    )


def _marketplace_ref(home: Path) -> tuple[str, bytes]:
    config = home / "config.toml"
    try:
        snapshot = config.read_bytes()
    except OSError as error:
        raise RuntimeError(
            "existing marketplace has no recoverable registration"
        ) from error
    contents = snapshot.decode("utf-8")
    section = re.search(
        r"(?ms)^\[(?:plugin_)?marketplaces\.codexy\]\s*$.*?(?=^\[|\Z)",
        contents,
    )
    match = (
        None
        if section is None
        else re.search(r'(?m)^ref\s*=\s*"([^"]+)"\s*$', section.group())
    )
    if match is None:
        raise RuntimeError("existing marketplace has no recoverable registration")
    return match.group(1), snapshot


def _restore_config(home: Path, snapshot: bytes) -> None:
    (home / "config.toml").write_bytes(snapshot)


def _find_codex() -> Path:
    candidate = shutil.which("codex")
    if not candidate:
        raise RuntimeError("official Codex CLI is not on PATH")
    path = Path(candidate).resolve(strict=True)
    if not path.is_absolute() or not path.is_file():
        raise RuntimeError(
            "official Codex CLI must resolve to an absolute regular file"
        )
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
    except (json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"{stage} returned invalid JSON") from error
