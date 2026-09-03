"""Read-only identity checks for an official marketplace checkout."""

from __future__ import annotations

import json
import os
import re
import subprocess
from pathlib import Path

from .plugin_resolution import OFFICIAL
from .updater import _validate_real_path


_REVISION = re.compile(r"[0-9a-f]{40}\Z")


def require_pinned_registration(
    home: Path, marketplace_root: Path, expected: str
) -> None:
    ref, _ = _marketplace_ref(home)
    if ref != expected:
        raise RuntimeError(
            "official marketplace registration is not pinned to the target"
        )

    metadata_path = marketplace_root / ".codex-marketplace-install.json"
    _validate_real_path(metadata_path, require_exists=False)
    expected_revision = _git_revision(marketplace_root, expected)
    if not metadata_path.exists():
        if _git_revision(marketplace_root, "HEAD") != expected_revision:
            raise RuntimeError(
                "official marketplace checkout revision is outside the expected release tag"
            )
        return
    try:
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        raise RuntimeError(
            "official marketplace install metadata is invalid"
        ) from error
    if not isinstance(metadata, dict):
        raise RuntimeError("official marketplace install metadata is invalid")
    if metadata.get("source_type") != "git" or metadata.get("source") != OFFICIAL:
        raise RuntimeError(
            "official marketplace install metadata has an invalid source"
        )
    if metadata.get("ref_name") != expected:
        raise RuntimeError(
            "official marketplace install metadata is not pinned to the target"
        )
    revision = metadata.get("revision")
    if not isinstance(revision, str) or not _REVISION.fullmatch(revision):
        raise RuntimeError(
            "official marketplace install metadata has an invalid revision"
        )
    if (
        revision != expected_revision
        or _git_revision(marketplace_root, "HEAD") != expected_revision
    ):
        raise RuntimeError(
            "official marketplace checkout revision is outside the expected release tag"
        )


def config_snapshot(home: Path) -> bytes:
    try:
        return (home / "config.toml").read_bytes()
    except FileNotFoundError:
        return b""


def _marketplace_ref(home: Path) -> tuple[str | None, bytes]:
    snapshot = config_snapshot(home)
    if not snapshot:
        raise RuntimeError("existing marketplace has no recoverable registration")
    section = re.search(
        r"(?ms)^\[(?:plugin_)?marketplaces\.codexy\]\s*$.*?(?=^\[|\Z)",
        snapshot.decode("utf-8"),
    )
    if section is None:
        raise RuntimeError("existing marketplace has no recoverable registration")
    match = re.search(r'(?m)^ref\s*=\s*"([^"]+)"\s*$', section.group())
    return (None if match is None else match.group(1)), snapshot


def _git_revision(root: Path, reference: str) -> str:
    environment = os.environ.copy()
    for name in (
        "GIT_DIR",
        "GIT_EXEC_PATH",
        "GIT_SSH",
        "GIT_SSH_COMMAND",
        "GIT_WORK_TREE",
        "SSH_ASKPASS",
    ):
        environment.pop(name, None)
    environment.update(
        {
            "GIT_CONFIG_COUNT": "0",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_TERMINAL_PROMPT": "0",
        }
    )
    result = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "--verify", f"{reference}^{{commit}}"],
        text=True,
        capture_output=True,
        check=False,
        env=environment,
    )
    revision = result.stdout.strip()
    if result.returncode or not _REVISION.fullmatch(revision):
        raise RuntimeError(
            "official marketplace cannot resolve the expected release revision"
        )
    return revision
