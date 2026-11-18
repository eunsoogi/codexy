"""Operation-bound local marketplace rollback identity, never a release receipt."""

from __future__ import annotations

import hashlib
import json
import stat
from dataclasses import dataclass
from pathlib import Path

from .component_manifest import ComponentManifest, valid_semver
from .component_transaction_snapshot import (
    _atomic_write,
    _read_regular,
    _unlink_regular,
)
from .component_transition_journal import Journal
from .local_marketplace import _unique_object, validate_local_archive
from .plugin_resolution import MarketplaceIdentity
from .updater import _validate_real_path

SCHEMA = "getcodexy.local-marketplace-upgrade.v1"


def tree_digest(root: Path) -> str:
    """Detect changes to the immutable old archive before any rollback execution."""
    _validate_real_path(root, require_exists=True)
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        metadata = path.lstat()
        relative = path.relative_to(root).as_posix().encode()
        if not (stat.S_ISREG(metadata.st_mode) or stat.S_ISDIR(metadata.st_mode)):
            raise ValueError("local archive contains a non-regular entry")
        digest.update(len(relative).to_bytes(8, "big") + relative)
        digest.update(str(stat.S_IMODE(metadata.st_mode)).encode() + b"\0")
        if path.is_file():
            _validate_real_path(path, require_exists=True)
            digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def path(home: Path) -> Path:
    return home / "getcodexy/local-marketplace-upgrade.json"


@dataclass(frozen=True)
class LocalUpgrade:
    operation_id: str
    old: MarketplaceIdentity
    target: MarketplaceIdentity
    old_version: str
    target_version: str
    old_digest: str
    target_digest: str

    def validate(
        self,
        home: Path,
        journal: Journal,
        manifest: ComponentManifest,
        *,
        target_contents: bool = True,
    ) -> None:
        names = tuple(c.plugin for c in manifest.components)
        if self.operation_id != journal.identifier or journal.command not in {
            "update",
            "bootstrap",
        }:
            raise ValueError("local upgrade journal does not match the operation")
        if (
            not valid_semver(self.old_version)
            or self.target_version != manifest.version
        ):
            raise ValueError("local upgrade journal has invalid versions")
        if tuple(map(int, self.old_version.split("."))) >= tuple(
            map(int, self.target_version.split("."))
        ):
            raise ValueError("local upgrade journal is not a forward transition")
        if (
            self.old.root == self.target.root
            or self.old.root in self.target.root.parents
            or self.target.root in self.old.root.parents
            or home / "getcodexy/marketplaces" not in self.target.root.parents
        ):
            raise ValueError("local upgrade roots are not separate managed archives")
        validate_local_archive(self.old.root, self.old_version, names)
        if target_contents:
            validate_local_archive(self.target.root, self.target_version, names)
            if tree_digest(self.target.root) != self.target_digest:
                raise ValueError("verified target marketplace changed during upgrade")
        if tree_digest(self.old.root) != self.old_digest:
            raise ValueError("prior local marketplace changed during upgrade")

    def write(self, home: Path) -> None:
        _atomic_write(
            path(home),
            json.dumps(
                {
                    "schema": SCHEMA,
                    "operation_id": self.operation_id,
                    "old_root": str(self.old.root),
                    "target_root": str(self.target.root),
                    "old_version": self.old_version,
                    "target_version": self.target_version,
                    "old_digest": self.old_digest,
                    "target_digest": self.target_digest,
                },
                sort_keys=True,
            ).encode(),
        )


def read(home: Path) -> LocalUpgrade | None:
    raw = _read_regular(path(home))
    if raw is None:
        return None
    value = json.loads(raw, object_pairs_hook=_unique_object)
    fields = {
        "schema",
        "operation_id",
        "old_root",
        "target_root",
        "old_version",
        "target_version",
        "old_digest",
        "target_digest",
    }
    if not isinstance(value, dict) or set(value) != fields or value["schema"] != SCHEMA:
        raise ValueError("invalid local upgrade journal shape")
    if any(not isinstance(item, str) for item in value.values()):
        raise ValueError("invalid local upgrade journal value")
    roots = []
    for key in ("old_root", "target_root"):
        root = Path(value[key])
        if not root.is_absolute() or str(root) != value[key] or ".." in root.parts:
            raise ValueError("invalid local upgrade root")
        roots.append(MarketplaceIdentity(root, "local", str(root)))
    return LocalUpgrade(
        value["operation_id"],
        *roots,
        value["old_version"],
        value["target_version"],
        value["old_digest"],
        value["target_digest"],
    )


def clear(home: Path) -> None:
    _unlink_regular(path(home))
