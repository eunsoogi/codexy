from __future__ import annotations

import argparse
import json
import os
import stat
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Literal


Mode = Literal["check", "install", "uninstall", "diagnose"]
MANAGED = b"# CODEXY MANAGED AGENT\n"
REPARSE_POINT = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)


@dataclass(frozen=True)
class SyncResult:
    mode: str
    status: str
    plugin_identity: str
    plugin_root: str
    codex_home: str
    changed: bool
    restart_required: bool
    diagnostics: tuple[str, ...]

    def as_dict(self) -> dict[str, object]:
        return asdict(self)


def _absolute(path: str | os.PathLike[str]) -> Path:
    absolute = Path(os.path.abspath(Path(path).expanduser()))
    if len(absolute.parts) < 2:
        return absolute
    top_level = Path(absolute.anchor) / absolute.parts[1]
    metadata = os.lstat(top_level)
    if _is_link_or_reparse(metadata):
        if os.name == "nt" or metadata.st_uid != 0:
            raise ValueError(f"trusted path boundary cannot be a reparse point: {top_level}")
        return top_level.resolve(strict=True).joinpath(*absolute.parts[2:])
    return absolute


def _is_link_or_reparse(metadata: os.stat_result) -> bool:
    return stat.S_ISLNK(metadata.st_mode) or bool(
        getattr(metadata, "st_file_attributes", 0) & REPARSE_POINT
    )


def _validate_real_path(path: Path, *, require_exists: bool) -> None:
    if not path.is_absolute():
        raise ValueError(f"path must be absolute: {path}")
    current = Path(path.anchor)
    missing = False
    for part in path.parts[1:]:
        current /= part
        if missing:
            continue
        try:
            metadata = os.lstat(current)
        except FileNotFoundError:
            missing = True
            continue
        if _is_link_or_reparse(metadata):
            raise ValueError(f"path must not traverse a symlink or reparse point: {current}")
        if current != path and not stat.S_ISDIR(metadata.st_mode):
            raise ValueError(f"path component must be a directory: {current}")
    if require_exists and missing:
        raise FileNotFoundError(path)


def _plugin_identity(plugin_root: Path) -> str:
    manifest = plugin_root / ".codex-plugin" / "plugin.json"
    _validate_real_path(manifest, require_exists=True)
    data = json.loads(manifest.read_text(encoding="utf-8"))
    identity = data.get("name") if isinstance(data, dict) else None
    if identity != "codexy":
        raise ValueError(f"plugin manifest identity must be 'codexy': {manifest}")
    return identity


def _regular_bytes(path: Path) -> bytes | None:
    try:
        metadata = os.lstat(path)
    except FileNotFoundError:
        return None
    if _is_link_or_reparse(metadata) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"agent projection must be a regular non-link file: {path}")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    try:
        opened = os.fstat(descriptor)
        if (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
            raise RuntimeError(f"agent projection changed while opening: {path}")
        with os.fdopen(descriptor, "rb", closefd=False) as source:
            return source.read()
    finally:
        os.close(descriptor)


def _check(plugin_root: Path, codex_home: Path, identity: str) -> SyncResult:
    packaged_root = plugin_root / "agents"
    _validate_real_path(packaged_root, require_exists=True)
    packaged = sorted(packaged_root.glob("codexy-*.toml"))
    if not packaged:
        raise ValueError(f"plugin has no packaged Codexy agents: {packaged_root}")
    projections_root = codex_home / "agents" / "codexy"
    _validate_real_path(projections_root, require_exists=False)
    expected = {path.name: MANAGED + (_regular_bytes(path) or b"") for path in packaged}
    observed: dict[str, bytes] = {}
    if projections_root.is_dir():
        for path in projections_root.glob("codexy-*.toml"):
            contents = _regular_bytes(path)
            if contents is not None and contents.startswith(MANAGED):
                observed[path.name] = contents
    ready = observed == expected
    diagnostic = "READY" if ready else "UPDATE_REQUIRED"
    return SyncResult(
        mode="check",
        status="ready" if ready else "update_required",
        plugin_identity=identity,
        plugin_root=str(plugin_root),
        codex_home=str(codex_home),
        changed=False,
        restart_required=False,
        diagnostics=(diagnostic,),
    )


def _registration(
    plugin_root: Path, codex_home: Path, identity: str, mode: Mode
) -> SyncResult:
    script = plugin_root / "skills/codex-orchestration/scripts/register-codexy-agents"
    _validate_real_path(script, require_exists=True)
    command = [
        sys.executable,
        "-B",
        str(script),
        "--plugin-root",
        str(plugin_root),
        "--codex-home",
        str(codex_home),
    ]
    if mode == "uninstall":
        command.append("--uninstall")
    elif mode == "diagnose":
        command.append("--diagnose")
    completed = subprocess.run(command, text=True, capture_output=True, check=False)
    diagnostics = tuple(
        line
        for text in (completed.stdout, completed.stderr)
        for line in text.splitlines()
        if line.strip()
    )
    status = "completed" if completed.returncode == 0 else "error"
    return SyncResult(
        mode=mode,
        status=status,
        plugin_identity=identity,
        plugin_root=str(plugin_root),
        codex_home=str(codex_home),
        changed=completed.returncode == 0 and mode in {"install", "uninstall"},
        restart_required=completed.returncode == 0 and mode == "install",
        diagnostics=diagnostics,
    )


def sync_agents(
    plugin_root: str | os.PathLike[str],
    codex_home: str | os.PathLike[str],
    mode: Mode,
) -> SyncResult:
    """Check or synchronize Codexy agents without writing to stdout or stderr."""
    if mode not in {"check", "install", "uninstall", "diagnose"}:
        raise ValueError(f"unsupported sync mode: {mode}")
    plugin = _absolute(plugin_root)
    home = _absolute(codex_home)
    _validate_real_path(plugin, require_exists=True)
    _validate_real_path(home, require_exists=False)
    identity = _plugin_identity(plugin)
    if mode == "check":
        return _check(plugin, home, identity)
    return _registration(plugin, home, identity, mode)


def main() -> int:
    parser = argparse.ArgumentParser(prog="codexy-update", allow_abbrev=False)
    parser.add_argument("--plugin-root", type=Path, required=True)
    parser.add_argument(
        "--codex-home",
        type=Path,
        default=Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")),
    )
    parser.add_argument("--mode", choices=("check", "install", "uninstall", "diagnose"), required=True)
    arguments = parser.parse_args()
    try:
        result = sync_agents(arguments.plugin_root, arguments.codex_home, arguments.mode)
    except Exception as error:
        print(json.dumps({"status": "error", "error": str(error)}, ensure_ascii=False))
        return 1
    print(json.dumps(result.as_dict(), ensure_ascii=False, sort_keys=True))
    return 0 if result.status not in {"error", "update_required"} else 1
