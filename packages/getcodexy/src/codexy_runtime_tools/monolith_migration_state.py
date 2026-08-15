"""Durable, no-follow recovery state for a monolith migration."""

from __future__ import annotations

import base64
import json
from dataclasses import dataclass
from pathlib import Path

from .activation_transaction import ActivationSnapshot, Entry
from .component_manifest import load_component_manifest, valid_semver
from .component_resolver import ComponentResolutionError, resolve_components
from .monolith_baseline import BASELINES
from .component_transaction_state import (
    _atomic_write,
    _read_regular,
    _unique_object,
    _unlink_regular,
)
from .updater import _absolute


_SCHEMA = "getcodexy.monolith-migration.v1"


@dataclass(frozen=True)
class MigrationJournal:
    source_version: str
    target_version: str
    selection: tuple[str, ...]
    snapshot: ActivationSnapshot
    phase: str = "prepared"

    @classmethod
    def capture(
        cls,
        home: Path,
        source_version: str,
        target_version: str,
        selection: tuple[str, ...],
    ) -> "MigrationJournal":
        return cls(
            source_version, target_version, selection, ActivationSnapshot.capture(home)
        )

    def encode(self) -> dict[str, object]:
        return {
            "schema": _SCHEMA,
            "source_version": self.source_version,
            "target_version": self.target_version,
            "selection": list(self.selection),
            "phase": self.phase,
            "snapshot": [
                {
                    "path": entry.relative.as_posix(),
                    "mode": entry.mode,
                    "data": None
                    if entry.data is None
                    else base64.b64encode(entry.data).decode(),
                }
                for entry in self.snapshot.entries
            ],
        }

    @classmethod
    def decode(cls, home: Path, value: object) -> "MigrationJournal":
        expected = {
            "schema",
            "source_version",
            "target_version",
            "selection",
            "phase",
            "snapshot",
        }
        if (
            not isinstance(value, dict)
            or set(value) != expected
            or value.get("schema") != _SCHEMA
        ):
            raise ValueError("monolith migration journal has an invalid shape")
        source, target, selection, phase, entries = (
            value.get("source_version"),
            value.get("target_version"),
            value.get("selection"),
            value.get("phase"),
            value.get("snapshot"),
        )
        if (
            not all(valid_semver(item) for item in (source, target))
            or source == target
            or source not in BASELINES
            or not isinstance(phase, str)
            or not isinstance(selection, list)
            or phase not in {"prepared", "activating", "rolling-back"}
            or not isinstance(entries, list)
        ):
            raise ValueError("monolith migration journal has invalid values")
        try:
            resolved = resolve_components(load_component_manifest(), tuple(selection))
        except (ComponentResolutionError, TypeError) as error:
            raise ValueError(
                "monolith migration journal has invalid selection"
            ) from error
        if tuple(selection) != resolved:
            raise ValueError("monolith migration journal has invalid selection")
        decoded = tuple(_entry(item) for item in entries)
        if len({entry.relative for entry in decoded}) != len(decoded):
            raise ValueError("monolith migration journal has duplicate snapshot paths")
        return cls(
            source, target, tuple(selection), ActivationSnapshot(home, decoded), phase
        )

    def with_phase(self, phase: str) -> "MigrationJournal":
        return MigrationJournal(
            self.source_version,
            self.target_version,
            self.selection,
            self.snapshot,
            phase,
        )


def journal_path(home: Path) -> Path:
    return _absolute(home) / "getcodexy" / "monolith-migration.json"


def read_journal(home: Path) -> MigrationJournal | None:
    contents = _read_regular(journal_path(home))
    return (
        None
        if contents is None
        else MigrationJournal.decode(
            home, json.loads(contents, object_pairs_hook=_unique_object)
        )
    )


def write_journal(home: Path, journal: MigrationJournal) -> None:
    _atomic_write(
        journal_path(home), json.dumps(journal.encode(), sort_keys=True).encode()
    )


def clear_journal(home: Path) -> None:
    _unlink_regular(journal_path(home))


def _entry(value: object) -> Entry:
    if not isinstance(value, dict) or set(value) != {"path", "mode", "data"}:
        raise ValueError("monolith migration journal has an invalid snapshot")
    path, mode, data = value.get("path"), value.get("mode"), value.get("data")
    relative = Path(path) if isinstance(path, str) else None
    if (
        relative is None
        or relative.is_absolute()
        or any(part in {"", ".", ".."} for part in relative.parts)
        or not _snapshot_path_allowed(relative)
        or not isinstance(mode, int)
        or not 0 <= mode <= 0o777
    ):
        raise ValueError("monolith migration journal has an unsafe snapshot")
    if data is None:
        return Entry(relative, None, mode)
    if not isinstance(data, str):
        raise ValueError("monolith migration journal has invalid snapshot data")
    try:
        return Entry(relative, base64.b64decode(data, validate=True), mode)
    except ValueError as error:
        raise ValueError(
            "monolith migration journal has invalid snapshot data"
        ) from error


def _snapshot_path_allowed(relative: Path) -> bool:
    parts = relative.parts
    return (
        relative == Path("config.toml")
        or (
            len(parts) >= 2
            and parts[:2] in {("agents", "codexy"), ("agents", "codexy-github")}
        )
        or (len(parts) == 1 and relative.name.startswith("config.toml.codexy-backup-"))
    )
