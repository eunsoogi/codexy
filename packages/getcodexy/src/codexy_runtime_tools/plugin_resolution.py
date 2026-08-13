from __future__ import annotations

from pathlib import Path

from .component_json import loads
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
    preflight_named_install(payload, marketplace_root, "codexy")


def preflight_named_install(payload: object, marketplace_root: Path, name: str) -> None:
    marketplace_root = _absolute(marketplace_root)
    _validate_real_path(marketplace_root, require_exists=True)
    entries = _enabled(payload, name)
    if len(entries) > 1:
        raise ValueError(_install_count_error(name, "zero or one"))
    if entries:
        _require_official(entries[0], name)
        _source_root(entries[0], marketplace_root, name)


def official_install(
    payload: object,
    marketplace_root: Path,
    distribution_version: str,
) -> tuple[Path, str]:
    return official_named_install(
        payload, marketplace_root, distribution_version, "codexy"
    )


def official_named_install(
    payload: object,
    marketplace_root: Path,
    distribution_version: str,
    name: str,
) -> tuple[Path, str]:
    marketplace_root = _absolute(marketplace_root)
    _validate_real_path(marketplace_root, require_exists=True)
    entries = _enabled(payload, name)
    if len(entries) != 1:
        raise ValueError(_install_count_error(name, "exactly one"))
    item = entries[0]
    _require_official(item, name)
    root = _source_root(item, marketplace_root, name)
    version = item.get("version")
    if not isinstance(version, str):
        raise ValueError(f"official {name} install has invalid metadata")
    if version != distribution_version:
        raise ValueError(f"{name} plugin version must match the getcodexy distribution")

    manifest = root / ".codex-plugin" / "plugin.json"
    _validate_real_path(manifest, require_exists=True)
    data = loads(manifest.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or (
        data.get("name"),
        data.get("repository"),
        data.get("version"),
    ) != (name, PLUGIN_REPOSITORY, version):
        raise ValueError(f"official {name} install identity does not match its manifest")
    return root, version


def _items(payload: object, key: str) -> list[object]:
    if not isinstance(payload, dict):
        return []
    value = payload.get(key)
    return value if isinstance(value, list) else []


def _codexy_enabled(payload: object) -> list[dict[str, object]]:
    return _enabled(payload, "codexy")


def _enabled(payload: object, name: str) -> list[dict[str, object]]:
    return [
        item
        for item in _items(payload, "installed")
        if isinstance(item, dict)
        and item.get("enabled") is True
        and (
            item.get("pluginId") == f"{name}@codexy"
            or item.get("name") == name
        )
    ]


def _require_official(item: dict[str, object], name: str = "codexy") -> None:
    source = item.get("source")
    if not (
        item.get("pluginId") == f"{name}@codexy"
        and item.get("name") == name
        and item.get("marketplaceName") == "codexy"
        and item.get("installed") is True
        and item.get("enabled") is True
        and isinstance(source, dict)
        and source.get("source") == "local"
        and item.get("marketplaceSource")
        == {"sourceType": "git", "source": OFFICIAL}
    ):
        raise ValueError(_install_count_error(name, "zero or one"))


def _source_root(item: dict[str, object], marketplace_root: Path, name: str = "codexy") -> Path:
    source = item.get("source")
    path_value = source.get("path") if isinstance(source, dict) else None
    if not isinstance(path_value, str):
        raise ValueError(f"official {name} install has invalid metadata")
    if not Path(path_value).is_absolute():
        raise ValueError(f"official {name} install path must be absolute")
    root = _absolute(path_value)
    expected = marketplace_root / "plugins" / name
    if root != expected:
        raise ValueError(f"official {name} install must be inside its marketplace root")
    _validate_real_path(root, require_exists=True)
    return root


def _install_count_error(name: str, count: str) -> str:
    label = "Codexy" if name == "codexy" else name
    return f"expected {count} enabled official {label} install"
