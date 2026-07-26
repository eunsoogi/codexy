from __future__ import annotations

import json
from pathlib import Path

from .updater import _absolute, _validate_real_path

OFFICIAL = "https://github.com/eunsoogi/codexy.git"
PLUGIN_REPOSITORY = "https://github.com/eunsoogi/codexy"


def named_marketplace(payload: object) -> bool:
    return any(
        isinstance(item, dict) and item.get("name") == "codexy"
        for item in _items(payload, "marketplaces")
    )


def official_marketplace(payload: object) -> Path:
    named = [
        item
        for item in _items(payload, "marketplaces")
        if isinstance(item, dict) and item.get("name") == "codexy"
    ]
    if len(named) != 1 or named[0].get("marketplaceSource") != {
        "sourceType": "git",
        "source": OFFICIAL,
    }:
        raise ValueError("expected exactly one official Codexy marketplace")
    root_value = named[0].get("root")
    if not isinstance(root_value, str):
        raise ValueError("official Codexy marketplace root is missing")
    if not Path(root_value).is_absolute():
        raise ValueError("official Codexy marketplace root must be absolute")
    root = _absolute(root_value)
    _validate_real_path(root, require_exists=True)
    return root


def preflight_install(payload: object, marketplace_root: Path) -> None:
    marketplace_root = _absolute(marketplace_root)
    _validate_real_path(marketplace_root, require_exists=True)
    entries = _codexy_enabled(payload)
    if len(entries) > 1:
        raise ValueError("expected zero or one enabled official Codexy install")
    if entries:
        _require_official(entries[0])
        _source_root(entries[0], marketplace_root)


def official_install(
    payload: object,
    marketplace_root: Path,
    distribution_version: str,
) -> tuple[Path, str]:
    marketplace_root = _absolute(marketplace_root)
    _validate_real_path(marketplace_root, require_exists=True)
    entries = _codexy_enabled(payload)
    if len(entries) != 1:
        raise ValueError("expected exactly one enabled official Codexy install")
    item = entries[0]
    _require_official(item)
    root = _source_root(item, marketplace_root)
    version = item.get("version")
    if not isinstance(version, str):
        raise ValueError("official Codexy install has invalid metadata")
    if version != distribution_version:
        raise ValueError("Codexy plugin version must match the getcodexy distribution")

    manifest = root / ".codex-plugin" / "plugin.json"
    _validate_real_path(manifest, require_exists=True)
    data = json.loads(manifest.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or (
        data.get("name"),
        data.get("repository"),
        data.get("version"),
    ) != ("codexy", PLUGIN_REPOSITORY, version):
        raise ValueError("official Codexy install identity does not match its manifest")
    return root, version


def _items(payload: object, key: str) -> list[object]:
    if not isinstance(payload, dict):
        return []
    value = payload.get(key)
    return value if isinstance(value, list) else []


def _codexy_enabled(payload: object) -> list[dict[str, object]]:
    return [
        item
        for item in _items(payload, "installed")
        if isinstance(item, dict)
        and item.get("enabled") is True
        and (
            item.get("pluginId") == "codexy@codexy"
            or item.get("name") == "codexy"
            or item.get("marketplaceName") == "codexy"
        )
    ]


def _require_official(item: dict[str, object]) -> None:
    source = item.get("source")
    if not (
        item.get("pluginId") == "codexy@codexy"
        and item.get("name") == "codexy"
        and item.get("marketplaceName") == "codexy"
        and item.get("installed") is True
        and item.get("enabled") is True
        and isinstance(source, dict)
        and source.get("source") == "local"
        and item.get("marketplaceSource")
        == {"sourceType": "git", "source": OFFICIAL}
    ):
        raise ValueError("expected zero or one enabled official Codexy install")


def _source_root(item: dict[str, object], marketplace_root: Path) -> Path:
    source = item.get("source")
    path_value = source.get("path") if isinstance(source, dict) else None
    if not isinstance(path_value, str):
        raise ValueError("official Codexy install has invalid metadata")
    if not Path(path_value).is_absolute():
        raise ValueError("official Codexy install path must be absolute")
    root = _absolute(path_value)
    expected = marketplace_root / "plugins" / "codexy"
    if root != expected:
        raise ValueError("official Codexy install must be inside its marketplace root")
    _validate_real_path(root, require_exists=True)
    return root
