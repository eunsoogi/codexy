"""Transactional, section-exact official marketplace repinning."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path
from typing import Callable

from .plugin_resolution import named_marketplace, official_marketplace


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]


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
    if named_marketplace(market):
        official_marketplace(market)
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
            return official_marketplace(market)
        except Exception:
            try:
                _add_marketplace(executable, invoke, previous_ref)
            finally:
                _restore_config(home, config_snapshot)
            raise
    _add_marketplace(executable, invoke, f"v{target_version}")
    market = _json(
        invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
        "marketplace list",
    )
    return official_marketplace(market)


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
    section = re.search(
        r"(?ms)^\[(?:plugin_)?marketplaces\.codexy\]\s*$.*?(?=^\[|\Z)",
        snapshot.decode("utf-8"),
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


def _json(done: subprocess.CompletedProcess[str], stage: str) -> object:
    if done.returncode:
        raise RuntimeError(f"{stage} failed")
    try:
        return json.loads(done.stdout)
    except (json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"{stage} returned invalid JSON") from error
