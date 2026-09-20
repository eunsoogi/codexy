from __future__ import annotations

import argparse
import json
import os
import stat
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path


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


def compare_managed_files(
    plugin: Path, codex_home: Path, component: str
) -> dict[str, object]:
    """Compare catalog-derived projections with standalone registration files."""
    from .component_integrity import MAX_COMPONENT_BYTES, _read_regular
    from .component_registration_files import _text
    from .component_registration_health import (
        MANAGED_MARKERS,
        MANAGED_ROOTS,
        _catalog_agent_files,
        registration_report,
        registration_role,
    )
    from .component_health import _registration_roles

    if component not in MANAGED_MARKERS:
        return registration_report(component, False, [], (), (), 0, "exact")
    marker, relative_root = MANAGED_MARKERS[component], MANAGED_ROOTS[component]

    try:
        names = _catalog_agent_files(_text(plugin / "agents/catalog.toml", plugin))
        expected = {
            name: (
                marker
                + _text(plugin / "agents" / name, plugin)
                .replace("\r\n", "\n")
                .replace("\r", "\n")
            ).encode()
            for name in names
        }
        home = _absolute(codex_home)
        root = home / relative_root
        metadata = root.lstat() if os.path.lexists(root) else None
        if (
            metadata is None
            or stat.S_ISLNK(metadata.st_mode)
            or not stat.S_ISDIR(metadata.st_mode)
        ):
            state = "missing" if metadata is None else "unmanaged-conflict"
            cause = (
                "registration directory is absent"
                if metadata is None
                else "registration directory is not a real directory"
            )
            return registration_report(
                component,
                metadata is not None,
                [
                    registration_role(name, state, cause, metadata is not None)
                    for name in names
                ],
                (),
                (),
                len(names),
                state,
            )
        roles, managed, unmanaged, state = _registration_roles(
            home, root, marker, expected, _read_regular, MAX_COMPONENT_BYTES
        )
        return registration_report(
            component, True, roles, managed, unmanaged, len(names), state
        )
    except (KeyError, OSError, UnicodeDecodeError, SyntaxError, ValueError) as error:
        return registration_report(
            component,
            True,
            [],
            (),
            (),
            0,
            "unmanaged-conflict",
            f"registration comparison failed: {error}",
        )


def _absolute(path: str | os.PathLike[str]) -> Path:
    absolute = Path(os.path.abspath(Path(path).expanduser()))
    if len(absolute.parts) < 2:
        return absolute
    top = Path(absolute.anchor) / absolute.parts[1]
    info = os.lstat(top)
    if stat.S_ISLNK(info.st_mode):
        if info.st_uid != 0:
            raise ValueError(f"trusted path boundary cannot be a symlink: {top}")
        return top.resolve(strict=True).joinpath(*absolute.parts[2:])
    return absolute


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
            info = os.lstat(current)
        except FileNotFoundError:
            missing = True
            continue
        if stat.S_ISLNK(info.st_mode):
            raise ValueError(f"path must not traverse a symlink: {current}")
        if current != path and not stat.S_ISDIR(info.st_mode):
            raise ValueError(f"path component must be a directory: {current}")
    if require_exists and missing:
        raise FileNotFoundError(path)


def _identity(root: Path) -> str:
    manifest = root / ".codex-plugin" / "plugin.json"
    _validate_real_path(manifest, require_exists=True)
    data = json.loads(manifest.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or data.get("name") != "codexy":
        raise ValueError("plugin manifest identity must be codexy")
    return "codexy"


def sync_agents(
    plugin_root: str | os.PathLike[str],
    codex_home: str | os.PathLike[str],
    mode: str,
) -> SyncResult:
    if mode not in {"check", "install", "uninstall", "diagnose"}:
        raise ValueError(f"unsupported sync mode: {mode}")

    root = _absolute(plugin_root)
    home = _absolute(codex_home)
    _validate_real_path(root, require_exists=True)
    _validate_real_path(home, require_exists=False)
    identity = _identity(root)
    script = root / "skills/orchestration/scripts/register_codexy_agents.py"
    _validate_real_path(script, require_exists=True)
    command = [sys.executable, "-B", str(script)] + [
        "--plugin-root",
        str(root),
        "--codex-home",
        str(home),
    ]
    if mode == "check" or mode == "diagnose":
        command.append("--diagnose")
    elif mode == "uninstall":
        command.append("--uninstall")

    environment = os.environ.copy()
    environment.pop("PYTHONHOME", None)
    environment.pop("PYTHONPATH", None)
    environment["PYTHONNOUSERSITE"] = "1"
    done = subprocess.run(
        command,
        text=True,
        capture_output=True,
        check=False,
        env=environment,
    )
    diagnostics = tuple(
        filter(str.strip, done.stdout.splitlines() + done.stderr.splitlines())
    )
    registration = compare_managed_files(root, home, "core")
    ready = (
        done.returncode == 0
        and registration["observed"]
        and registration["state"] == "exact"
        and (
            mode not in {"check", "diagnose"}
            or any(line.startswith("A role-discovery: PASS") for line in diagnostics)
        )
    )
    status = _status_for(mode, done.returncode, ready)
    changed = done.returncode == 0 and mode in {"install", "uninstall"}
    restart = done.returncode == 0 and mode == "install"
    return SyncResult(
        mode, status, identity, str(root), str(home), changed, restart, diagnostics
    )


def _status_for(mode: str, returncode: int, ready: bool) -> str:
    if mode == "check":
        return "ready" if ready else "update_required"
    return "completed" if returncode == 0 and (mode != "install" or ready) else "error"


def main() -> int:
    parser = argparse.ArgumentParser(prog="codexy-update", allow_abbrev=False)
    parser.add_argument("--plugin-root", type=Path)
    parser.add_argument(
        "--codex-home",
        type=Path,
        default=Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")),
    )
    parser.add_argument("--mode", choices=("check", "install", "uninstall", "diagnose"))
    parser.add_argument("--pre-session", action="store_true")
    args = parser.parse_args()
    try:
        if args.pre_session:
            if args.plugin_root or args.mode:
                parser.error("--pre-session does not accept --plugin-root or --mode")
            from .pre_session import run_pre_session

            result = run_pre_session(args.codex_home)
            if result.changed:
                print(
                    f"Codexy {result.version} agent projections synchronized. Start Codex.",
                    file=sys.stderr,
                )
            return 0

        if not args.plugin_root or not args.mode:
            parser.error(
                "--plugin-root and --mode are required unless --pre-session is used"
            )
        result = sync_agents(args.plugin_root, args.codex_home, args.mode)
        print(json.dumps(result.as_dict(), sort_keys=True))
        return 0 if result.status not in {"error", "update_required"} else 1
    except Exception as error:
        print(f"codexy update: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
