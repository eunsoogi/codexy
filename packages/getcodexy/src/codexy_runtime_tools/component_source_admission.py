"""Read-only filesystem provenance checks for resolver-admitted component roots."""

from __future__ import annotations

from pathlib import Path

from .component_manifest import Component


def trusted_component_root(marketplace_root: Path, component: Component) -> bool:
    """Require a local, real, non-reparse component tree before diagnostic reads."""
    if _network_path(marketplace_root):
        return False
    root = marketplace_root / component.asset.package_root
    try:
        ancestry = all(_local_directory(path) for path in _ancestry(marketplace_root, root))
        contained = root.resolve(strict=True).is_relative_to(marketplace_root.resolve(strict=True))
        return ancestry and contained
    except (OSError, RuntimeError):
        return False


def _ancestry(marketplace_root: Path, root: Path) -> tuple[Path, ...]:
    current, result = root, []
    while True:
        result.append(current)
        if current == marketplace_root:
            return tuple(result)
        current = current.parent


def _local_directory(path: Path) -> bool:
    junction = getattr(path, "is_junction", lambda: False)
    return path.is_dir() and not path.is_symlink() and not junction()


def _network_path(path: Path) -> bool:
    return str(path).replace("\\", "/").startswith("//")
