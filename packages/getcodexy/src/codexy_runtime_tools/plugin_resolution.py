from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from .local_marketplace import validate_local_archive
from .updater import _absolute, _validate_real_path

OFFICIAL = "https://github.com/eunsoogi/codexy.git"
PLUGIN_REPOSITORY = "https://github.com/eunsoogi/codexy"


@dataclass(frozen=True)
class MarketplaceIdentity:
    root: Path
    source_type: str
    source_value: str

    @property
    def host_source(self) -> dict[str, str]:
        return {"sourceType": self.source_type, "source": self.source_value}


MarketplaceBinding = Path | MarketplaceIdentity


def named_marketplace(payload: object) -> bool:
    return any(
        isinstance(item, dict) and item.get("name") == "codexy"
        for item in _items(payload, "marketplaces")
    )


def marketplace_identity(payload: object) -> MarketplaceIdentity:
    named = [
        item
        for item in _items(payload, "marketplaces")
        if isinstance(item, dict) and item.get("name") == "codexy"
    ]
    if len(named) != 1:
        raise ValueError("expected exactly one Codexy marketplace")
    item = named[0]
    root_value = item.get("root")
    if not isinstance(root_value, str):
        raise ValueError("Codexy marketplace root is missing")
    if not Path(root_value).is_absolute():
        raise ValueError("Codexy marketplace root must be absolute")
    root = _absolute(root_value)
    _validate_real_path(root, require_exists=True)
    source = item.get("marketplaceSource")
    if not isinstance(source, dict) or set(source) != {"sourceType", "source"}:
        raise ValueError("Codexy marketplace source is missing or invalid")
    source_type, source_value = source.get("sourceType"), source.get("source")
    if not isinstance(source_type, str) or not isinstance(source_value, str):
        raise ValueError("Codexy marketplace source is missing or invalid")
    if source_type == "git" and source_value == OFFICIAL:
        return MarketplaceIdentity(root, source_type, source_value)
    if source_type != "local":
        raise ValueError("Codexy marketplace source is foreign")
    if (
        not Path(source_value).is_absolute()
        or source_value != str(root)
        or _absolute(source_value) != root
    ):
        raise ValueError("local Codexy marketplace source must match its root")
    return MarketplaceIdentity(root, source_type, source_value)


def official_marketplace(payload: object) -> Path:
    try:
        identity = marketplace_identity(payload)
    except (OSError, ValueError, RuntimeError) as error:
        raise ValueError("expected exactly one official Codexy marketplace") from error
    if identity.host_source != {"sourceType": "git", "source": OFFICIAL}:
        raise ValueError("expected exactly one official Codexy marketplace")
    return identity.root


def marketplace_path(binding: MarketplaceBinding) -> Path:
    return (
        binding.root
        if isinstance(binding, MarketplaceIdentity)
        else Path(binding)
    )


def marketplace_source(binding: MarketplaceBinding) -> dict[str, str]:
    return (
        binding.host_source
        if isinstance(binding, MarketplaceIdentity)
        else {"sourceType": "git", "source": OFFICIAL}
    )


def validate_local_marketplace(
    binding: MarketplaceBinding,
    version: str | None = None,
    plugin_names: tuple[str, ...] | None = None,
) -> None:
    """Validate existing archive metadata before a local host mutation."""
    if not isinstance(binding, MarketplaceIdentity) or binding.source_type != "local":
        return
    validate_local_archive(binding.root, version, plugin_names)


def preflight_install(
    payload: object,
    marketplace_root: MarketplaceBinding,
    *,
    version: str | None = None,
    plugin_names: tuple[str, ...] | None = None,
) -> None:
    validate_local_marketplace(marketplace_root, version, plugin_names)
    preflight_named_install(payload, marketplace_root, "codexy")


def preflight_named_install(
    payload: object, marketplace_root: MarketplaceBinding, name: str
) -> None:
    root = (
        marketplace_root.root
        if isinstance(marketplace_root, MarketplaceIdentity)
        else _absolute(marketplace_root)
    )
    _validate_real_path(root, require_exists=True)
    entries = _enabled(payload, name)
    if len(entries) > 1:
        raise ValueError(_install_count_error(name, "zero or one"))
    if entries:
        _require_install(entries[0], name, marketplace_source(marketplace_root))
        _source_root(entries[0], root, name)


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
    return named_install(payload, marketplace_root, distribution_version, name)


def named_install(
    payload: object,
    marketplace_root: MarketplaceBinding,
    distribution_version: str,
    name: str,
) -> tuple[Path, str]:
    root = (
        marketplace_root.root
        if isinstance(marketplace_root, MarketplaceIdentity)
        else _absolute(marketplace_root)
    )
    _validate_real_path(root, require_exists=True)
    entries = _enabled(payload, name)
    if len(entries) != 1:
        raise ValueError(_install_count_error(name, "exactly one"))
    item = entries[0]
    _require_install(item, name, marketplace_source(marketplace_root))
    root = _source_root(item, root, name)
    version = item.get("version")
    if not isinstance(version, str):
        raise ValueError(f"official {name} install has invalid metadata")
    if version != distribution_version:
        raise ValueError(f"{name} plugin version must match the getcodexy distribution")

    manifest = root / ".codex-plugin" / "plugin.json"
    _validate_real_path(manifest, require_exists=True)
    data = json.loads(manifest.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or (
        data.get("name"),
        data.get("repository"),
        data.get("version"),
    ) != (name, PLUGIN_REPOSITORY, version):
        raise ValueError(
            f"official {name} install identity does not match its manifest"
        )
    return root, version


def _items(payload: object, key: str) -> list[object]:
    if not isinstance(payload, dict):
        return []
    value = payload.get(key)
    return value if isinstance(value, list) else []


def _enabled(payload: object, name: str) -> list[dict[str, object]]:
    return [
        item
        for item in _items(payload, "installed")
        if isinstance(item, dict)
        and item.get("enabled") is True
        and (item.get("pluginId") == f"{name}@codexy" or item.get("name") == name)
    ]


def _require_install(
    item: dict[str, object], name: str, expected_source: dict[str, str]
) -> None:
    source = item.get("source")
    if not (
        item.get("pluginId") == f"{name}@codexy"
        and item.get("name") == name
        and item.get("marketplaceName") == "codexy"
        and item.get("installed") is True
        and item.get("enabled") is True
        and isinstance(source, dict)
        and source.get("source") == "local"
        and item.get("marketplaceSource") == expected_source
    ):
        raise ValueError(_install_count_error(name, "zero or one"))


def _source_root(
    item: dict[str, object], marketplace_root: Path, name: str = "codexy"
) -> Path:
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
