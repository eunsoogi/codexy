"""Materialize the core Watcher entrypoint registered in ``.mcp.json``."""

from __future__ import annotations

import os
import shutil
import stat
import tempfile
from pathlib import Path

from .installer import executable, install_package
from .runtime_configuration import Configuration
from .updater import _absolute, _validate_real_path


WATCHER_COMMAND = Path("mcp/codexy-mcp-watcher")
WATCHER_SOURCE = Path("mcp/codexy-mcp-watcher.sh")
WATCHER_WINDOWS = Path("mcp/codexy-mcp-watcher.exe")
WATCHER_RUNTIME = Path("runtime/codexy-mcp-watcher-windows-x86_64.exe")


def watcher_entrypoint(plugin: Path) -> Path:
    """Return the platform target that the registered Watcher command resolves to."""
    relative = WATCHER_WINDOWS if os.name == "nt" else WATCHER_COMMAND
    return plugin / relative


def materialize_watcher(plugin: Path, home: Path | None = None) -> Path:
    """Create the installed platform target for the common Watcher command."""
    target = watcher_entrypoint(plugin)
    if os.name == "nt":
        source = _windows_source(plugin)
        if source is not None:
            _publish(source, target)
        elif _valid_executable(target):
            return target
        else:
            _install_windows_runtime(plugin, target, home)
    else:
        source = plugin / WATCHER_SOURCE
        _require_executable(source, "core Watcher source launcher")
        _publish(source, target)
    _require_executable(target, "materialized core Watcher entrypoint")
    return target


def valid_watcher_entrypoint(plugin: Path) -> bool:
    """Check the exact installed target without changing the plugin."""
    return _valid_executable(watcher_entrypoint(plugin))


def _windows_source(plugin: Path) -> Path | None:
    runtime_dir = os.environ.get("CODEXY_RUNTIME_DIR")
    if runtime_dir:
        path = Path(runtime_dir)
        if not path.is_absolute():
            raise RuntimeError(f"CODEXY_RUNTIME_DIR must be absolute: {path}")
        candidate = path / WATCHER_RUNTIME.name
        if _valid_executable(candidate):
            return candidate
    bundled = plugin / WATCHER_RUNTIME
    return bundled if _valid_executable(bundled) else None


def _install_windows_runtime(plugin: Path, target: Path, home: Path | None) -> None:
    try:
        config = Configuration.load("watcher", plugin, ["--stdio"])
    except SystemExit as error:
        raise RuntimeError("core Watcher runtime configuration is invalid") from error
    if not config.runtime_name.endswith(".exe"):
        raise RuntimeError("core Watcher Windows runtime is not an executable")
    cache_parent = (home / "getcodexy") if home is not None else None
    if cache_parent is not None:
        cache_parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix="watcher-package-", dir=str(cache_parent) if cache_parent else None
    ) as temporary:
        work = Path(temporary)
        staged = work / config.runtime_name
        install_package(config, work, staged)
        _publish(staged, target)


def _publish(source: Path, target: Path) -> None:
    _require_executable(source, "Watcher runtime source")
    _validate_real_path(_absolute(target.parent), require_exists=False)
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary = target.with_name(f".{target.name}.{os.getpid()}.tmp")
    _validate_real_path(_absolute(temporary), require_exists=False)
    try:
        shutil.copyfile(source, temporary)
        temporary.chmod(stat.S_IMODE(source.stat().st_mode) & 0o777)
        if not _valid_executable(temporary):
            raise RuntimeError(
                f"materialized Watcher entrypoint is not executable: {temporary}"
            )
        os.replace(temporary, target)
    finally:
        temporary.unlink(missing_ok=True)


def _require_executable(path: Path, label: str) -> None:
    if not _valid_executable(path):
        raise RuntimeError(f"{label} is missing or not executable: {path}")


def _valid_executable(path: Path) -> bool:
    try:
        metadata = path.lstat()
    except OSError:
        return False
    return (
        stat.S_ISREG(metadata.st_mode)
        and not stat.S_ISLNK(metadata.st_mode)
        and executable(path)
    )
