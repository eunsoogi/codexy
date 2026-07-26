"""Immutable, standalone runtime-release contract validation."""

from __future__ import annotations

import hashlib
import json
import re
import tarfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .identity import Compatibility, compatibility, digest, document, object, platforms, string


REPOSITORY = "https://github.com/eunsoogi/codexy"
RELEASE_SCHEMA = "codexy-runtime-release/v1"
CANDIDATE_SCHEMA = "codexy-runtime-candidate/v1"
_COMMIT = re.compile(r"[0-9a-f]{40}\Z")


@dataclass(frozen=True)
class Source:
    repository: str
    commit: str


@dataclass(frozen=True)
class Artifact:
    tag: str
    url: str
    sha256: str
    payload_manifest_sha256: str


@dataclass(frozen=True)
class RuntimeRelease:
    state: str
    source: Source
    artifact: Artifact
    compatibility: Compatibility
    platforms: dict[str, dict[str, dict[str, str]]]

    def advertises(self, *, platform: str) -> bool:
        return platform in self.platforms

    def supports(self, *, server: str, platform: str, bootstrap_api: int,
                 plugin_runtime_api: int, transport: str, mcp_protocol: str) -> bool:
        return (
            server in self.platforms.get(platform, {})
            and self.compatibility == Compatibility(
                bootstrap_api, plugin_runtime_api, transport, mcp_protocol
            )
        )

    def cache_key(self, *, platform: str, server: str) -> str:
        return "v3-" + hashlib.sha256(_encoded(self.identity(platform=platform, server=server))).hexdigest()

    def identity(self, *, platform: str, server: str) -> dict[str, Any]:
        binary = self.platforms.get(platform, {}).get(server)
        if binary is None:
            raise ValueError("runtime release does not advertise the selected binary")
        return {"schema": RELEASE_SCHEMA, "state": self.state, "source": self.source.__dict__,
                "artifact": self.artifact.__dict__, "compatibility": self.compatibility.__dict__,
                "platform": platform, "server": server, "binarySha256": binary["sha256"]}

    def marker(self, *, platform: str, server: str, binary_sha256: str) -> dict[str, Any]:
        return {"schema": "codexy-runtime-marker/v1", "identity": self.identity(platform=platform, server=server),
                "installedBinarySha256": binary_sha256}

    def valid_marker(self, marker: Any, *, platform: str, server: str, binary: bytes) -> bool:
        return marker == self.marker(platform=platform, server=server,
            binary_sha256=hashlib.sha256(binary).hexdigest())

    def verify_archive(self, archive: Path, *, platform: str) -> bool:
        if self.state == "legacy-public":
            return True
        try:
            with tarfile.open(archive, "r:gz") as package:
                names = [member.name for member in package.getmembers()]
                if len({name.casefold() for name in names}) != len(names):
                    raise ValueError("runtime archive has duplicate or casefold paths")
                package.getmember("plugins/codexy/.codex-plugin/plugin.json")
                candidate = document(package.extractfile("plugins/codexy/runtime-candidate.json").read())
                if _canonical(candidate) != self.artifact.payload_manifest_sha256:
                    raise ValueError("runtime candidate digest does not match release")
                _validate_candidate(candidate, self, package, platform)
        except (AttributeError, KeyError, OSError, tarfile.TarError, TypeError, json.JSONDecodeError) as error:
            raise ValueError(f"invalid runtime candidate: {error}") from error
        return True


def _canonical(value: Any) -> str:
    return hashlib.sha256(_encoded(value)).hexdigest()


def _encoded(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def load(plugin_root: Path) -> RuntimeRelease:
    path = plugin_root / "runtime-release.json"
    try:
        value = document(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"runtime release is missing or invalid: {error}") from error
    value = object(value, "document")
    if set(value) != {"schema", "state", "source", "artifact", "compatibility", "platforms"}:
        raise ValueError("runtime release has unknown or missing fields")
    if value.get("schema") != RELEASE_SCHEMA:
        raise ValueError("runtime release schema must be codexy-runtime-release/v1")
    state = value.get("state")
    if state not in {"legacy-public", "candidate-proven"}:
        raise ValueError("runtime release state must be legacy-public or candidate-proven")
    source = object(value.get("source"), "source")
    if set(source) != {"repository", "commit"}:
        raise ValueError("runtime release source has unknown or missing fields")
    commit = string(source.get("commit"), "source.commit")
    if source.get("repository") != REPOSITORY or not _COMMIT.fullmatch(commit):
        raise ValueError("runtime release source must use the canonical repository and lowercase commit")
    artifact = object(value.get("artifact"), "artifact")
    if set(artifact) != {"tag", "url", "sha256", "payloadManifestSha256"}:
        raise ValueError("runtime release artifact has unknown or missing fields")
    tag = string(artifact.get("tag"), "artifact.tag")
    url = string(artifact.get("url"), "artifact.url")
    if url != f"{REPOSITORY}/releases/download/{tag}/codexy-marketplace-plugin.tar.gz":
        raise ValueError("runtime release artifact URL is not canonical")
    return RuntimeRelease(state, Source(REPOSITORY, commit), Artifact(tag, url,
        digest(artifact.get("sha256"), "artifact.sha256"),
        digest(artifact.get("payloadManifestSha256"), "artifact.payloadManifestSha256")),
        compatibility(value.get("compatibility")),
        platforms(value.get("platforms"), require_path=state == "candidate-proven"))


def _validate_candidate(candidate: Any, release: RuntimeRelease, package: tarfile.TarFile, platform: str) -> None:
    candidate = object(candidate, "candidate")
    if release.state != "candidate-proven":
        raise ValueError("legacy runtime release has no candidate payload")
    if set(candidate) != {"schema", "source", "artifact", "compatibility", "platforms"} or candidate.get("schema") != CANDIDATE_SCHEMA or candidate.get("source") != {"repository": release.source.repository, "commit": release.source.commit}:
        raise ValueError("runtime candidate identity does not match release")
    if candidate.get("artifact") != {"tag": release.artifact.tag} or compatibility(candidate.get("compatibility")) != release.compatibility:
        raise ValueError("runtime candidate metadata does not match release")
    inventory = platforms(candidate.get("platforms"), require_path=True)
    if inventory != release.platforms or platform not in inventory:
        raise ValueError("runtime candidate inventory does not match release")
    for binary in inventory[platform].values():
        member = package.extractfile(f"plugins/codexy/{binary['path']}")
        if member is None or hashlib.sha256(member.read()).hexdigest() != binary["sha256"]:
            raise ValueError("runtime candidate binary digest does not match")
