"""Validation of the existing provenance files in a frozen local marketplace."""

from __future__ import annotations

import json
from pathlib import Path

from .updater import _validate_real_path


PLUGIN_REPOSITORY = "https://github.com/eunsoogi/codexy"


def validate_local_archive(
    root: Path,
    version: str | None = None,
    plugin_names: tuple[str, ...] | None = None,
) -> None:
    metadata_path = root / ".agents" / "plugins" / "marketplace.json"
    _validate_real_path(metadata_path, require_exists=True)
    try:
        metadata = json.loads(
            metadata_path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object
        )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        raise ValueError("local Codexy marketplace provenance is invalid") from error
    entries = metadata.get("plugins") if isinstance(metadata, dict) else None
    if not isinstance(metadata, dict) or metadata.get("name") != "codexy":
        raise ValueError("local Codexy marketplace provenance is invalid")
    if not isinstance(entries, list) or not entries:
        raise ValueError("local Codexy marketplace provenance is incomplete")
    names = tuple(
        entry.get("name")
        for entry in entries
        if isinstance(entry, dict) and isinstance(entry.get("name"), str)
    )
    expected_names = plugin_names or names
    if (
        len(names) != len(entries)
        or len(names) != len(set(names))
        or tuple(sorted(names)) != tuple(sorted(expected_names))
        or any(
            not name or name in {".", ".."} or Path(name).name != name for name in names
        )
    ):
        raise ValueError("local Codexy marketplace provenance is incomplete")
    for entry in entries:
        name = entry["name"]
        if version is not None and entry.get("version") != version:
            raise ValueError("local Codexy marketplace provenance version mismatch")
        if entry.get("source") != {
            "source": "local",
            "path": f"./plugins/{name}",
        }:
            raise ValueError("local Codexy marketplace provenance path mismatch")
        manifest = root / "plugins" / name / ".codex-plugin" / "plugin.json"
        _validate_real_path(manifest, require_exists=True)
        try:
            data = json.loads(
                manifest.read_text(encoding="utf-8"), object_pairs_hook=_unique_object
            )
        except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
            raise ValueError("local Codexy plugin provenance is invalid") from error
        if not isinstance(data, dict) or (
            data.get("name"),
            data.get("repository"),
            data.get("version"),
        ) != (name, PLUGIN_REPOSITORY, entry.get("version")):
            raise ValueError("local Codexy plugin provenance is inconsistent")


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("local Codexy marketplace provenance has duplicate keys")
        result[key] = value
    return result
