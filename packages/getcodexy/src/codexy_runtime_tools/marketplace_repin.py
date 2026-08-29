"""Transactional, section-exact official marketplace repinning."""

from __future__ import annotations

from base64 import b64encode
import json
import re
import subprocess
from pathlib import Path
from typing import Callable

from .component_transaction_state import _atomic_write
from .marketplace_identity import config_snapshot, require_pinned_registration
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
        previous_ref, prior_snapshot = _marketplace_ref(home)
        expected_ref = f"v{target_version}"
        _json(
            invoke(
                [str(executable), "plugin", "marketplace", "remove", "codexy", "--json"]
            ),
            "marketplace remove",
        )
        try:
            _add_marketplace(executable, invoke, expected_ref)
            market = _json(
                invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
                "marketplace list",
            )
            root = official_marketplace(market)
            require_pinned_registration(home, root, expected_ref)
            return root
        except Exception:
            if previous_ref is None or previous_ref == "main":
                reason = (
                    "unsafe-default-ref" if previous_ref is None else "unsafe-main-ref"
                )
                _quarantine_unsafe_registration(
                    executable, invoke, home, prior_snapshot, reason
                )
                raise RuntimeError(
                    "unsafe marketplace registration was removed; "
                    "recover the recorded configuration only after an explicit pinned repair"
                ) from None
            try:
                _add_marketplace(executable, invoke, previous_ref)
            finally:
                _restore_config(home, prior_snapshot)
            raise
    snapshot = config_snapshot(home)
    expected_ref = f"v{target_version}"
    try:
        _add_marketplace(executable, invoke, expected_ref)
        market = _json(
            invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
            "marketplace list",
        )
        root = official_marketplace(market)
        require_pinned_registration(home, root, expected_ref)
        return root
    except Exception as error:
        _quarantine_unsafe_registration(
            executable, invoke, home, snapshot, "marketplace-snapshot-drift"
        )
        raise RuntimeError(
            "marketplace snapshot was quarantined because its release tag pin could not be verified"
        ) from error


def _add_marketplace(executable: Path, invoke: Runner, ref: str) -> None:
    command = [
        str(executable),
        "plugin",
        "marketplace",
        "add",
        "eunsoogi/codexy",
    ]
    command.extend(("--ref", ref))
    command.append("--json")
    _json(
        invoke(command),
        "marketplace add",
    )


def _marketplace_ref(home: Path) -> tuple[str | None, bytes]:
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
    if section is None:
        raise RuntimeError("existing marketplace has no recoverable registration")
    match = re.search(r'(?m)^ref\s*=\s*"([^"]+)"\s*$', section.group())
    return (None if match is None else match.group(1)), snapshot


def _restore_config(home: Path, snapshot: bytes) -> None:
    _atomic_write(home / "config.toml", snapshot)


def quarantine_marketplace_drift(executable: Path, invoke: Runner, home: Path) -> None:
    _quarantine_unsafe_registration(
        executable,
        invoke,
        home,
        config_snapshot(home),
        "post-upgrade-marketplace-drift",
    )


def _quarantine_unsafe_registration(
    executable: Path, invoke: Runner, home: Path, snapshot: bytes, reason: str
) -> None:
    try:
        _json(
            invoke(
                [str(executable), "plugin", "marketplace", "remove", "codexy", "--json"]
            ),
            "marketplace quarantine remove",
        )
    finally:
        _write_recovery(home, snapshot, reason)
        _remove_official_registration(home, snapshot)


def _write_recovery(home: Path, snapshot: bytes, reason: str) -> None:
    receipt = home / "getcodexy" / "marketplace-recovery.json"
    _atomic_write(
        receipt,
        json.dumps(
            {
                "schema": "getcodexy.marketplace-recovery.v1",
                "reason": reason,
                "config_toml_base64": b64encode(snapshot).decode(),
            },
            sort_keys=True,
        ).encode(),
    )


def _remove_official_registration(home: Path, snapshot: bytes) -> None:
    section = re.compile(r"(?ms)^\[(?:plugin_)?marketplaces\.codexy\]\s*$.*?(?=^\[|\Z)")
    contents = snapshot.decode("utf-8")
    if section.search(contents) is None:
        return
    _atomic_write(home / "config.toml", section.sub("", contents).encode())


def _json(done: subprocess.CompletedProcess[str], stage: str) -> object:
    if done.returncode:
        raise RuntimeError(f"{stage} failed")
    try:
        return json.loads(done.stdout)
    except (json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"{stage} returned invalid JSON") from error
