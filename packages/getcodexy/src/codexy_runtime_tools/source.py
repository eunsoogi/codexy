"""Runtime package source modes and their independent identity contracts."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Any

from .contract import RuntimeRelease


class RuntimeSourceMode(str, Enum):
    SELECTED_RELEASE = "selected-release"
    EXPLICIT_OVERRIDE = "explicit-override"
    LEGACY_DEFAULT = "legacy-default"


@dataclass(frozen=True)
class RuntimeSourceIdentity:
    mode: RuntimeSourceMode
    package_sha256: str
    descriptor: dict[str, str]
    release: RuntimeRelease | None = None

    @classmethod
    def create(cls, *, override: bool, package_sha256: str, package_path: str,
               package_url: str, artifacts_api: str,
               release: RuntimeRelease | None) -> "RuntimeSourceIdentity":
        if override:
            kind, value = next((kind, value) for kind, value in (
                ("path", package_path), ("url", package_url), ("artifacts-api", artifacts_api)
            ) if value)
            return cls(RuntimeSourceMode.EXPLICIT_OVERRIDE, package_sha256,
                       {"kind": kind, "value": value})
        if release:
            return cls(RuntimeSourceMode.SELECTED_RELEASE, release.artifact.sha256,
                       {"tag": release.artifact.tag, "url": release.artifact.url}, release)
        return cls(RuntimeSourceMode.LEGACY_DEFAULT, package_sha256,
                   {"kind": "public-plugin-release", "value": package_url})

    def verify_archive(self, archive: Path, *, platform: str) -> None:
        if self.mode is RuntimeSourceMode.SELECTED_RELEASE:
            assert self.release is not None
            self.release.verify_archive(archive, platform=platform)

    def cache_key(self, *, platform: str, server: str) -> str | None:
        if self.mode is RuntimeSourceMode.SELECTED_RELEASE:
            assert self.release is not None
            return self.release.cache_key(platform=platform, server=server)
        if self.mode is RuntimeSourceMode.LEGACY_DEFAULT:
            return None
        return "v3-" + hashlib.sha256(self._encoded_identity(platform, server)).hexdigest()

    def marker(self, *, platform: str, server: str, binary_sha256: str) -> dict[str, Any] | None:
        if self.mode is RuntimeSourceMode.SELECTED_RELEASE:
            assert self.release is not None
            return self.release.marker(platform=platform, server=server,
                                       binary_sha256=binary_sha256)
        if self.mode is RuntimeSourceMode.LEGACY_DEFAULT:
            return None
        return {"schema": "codexy-runtime-override-marker/v1",
                "identity": self._identity(platform, server),
                "installedBinarySha256": binary_sha256}

    def valid_marker(self, marker: Any, *, platform: str, server: str, binary: bytes) -> bool:
        expected = self.marker(platform=platform, server=server,
                               binary_sha256=hashlib.sha256(binary).hexdigest())
        return expected is not None and marker == expected

    def _identity(self, platform: str, server: str) -> dict[str, Any]:
        return {"mode": self.mode.value, "packageSha256": self.package_sha256,
                "source": self.descriptor, "platform": platform, "server": server}

    def _encoded_identity(self, platform: str, server: str) -> bytes:
        return json.dumps(self._identity(platform, server), sort_keys=True,
                          separators=(",", ":")).encode()
