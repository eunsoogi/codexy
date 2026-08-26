"""Immutable, standalone runtime-release contract validation."""

from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .identity import (
    Compatibility,
    compatibility,
    digest,
    document,
    object,
    platforms,
    string,
)
from .release_contract_validation import (
    canonical_digest,
    encoded as _encoded,
    source_platforms,
    validate_classes,
    validate_provenance,
    verify_archive,
)

REPOSITORY = "https://github.com/eunsoogi/codexy"
RELEASE_SCHEMA = "codexy-runtime-release/v1"
_COMMIT = re.compile(r"[0-9a-f]{40}\Z")


def _canonical(value: Any) -> str:
    return canonical_digest(value)


@dataclass(frozen=True)
class Source:
    repository: str
    commit: str
    tree: str | None = None


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
    provenance: dict[str, Any] | None = None
    classes: dict[str, Any] | None = None

    def advertises(self, *, platform: str) -> bool:
        return platform in self.platforms

    def supports(
        self,
        *,
        server: str,
        platform: str,
        bootstrap_api: int,
        plugin_runtime_api: int,
        transport: str,
        mcp_protocol: str,
    ) -> bool:
        return server in self.platforms.get(
            platform, {}
        ) and self.compatibility == Compatibility(
            bootstrap_api, plugin_runtime_api, transport, mcp_protocol
        )

    def cache_key(self, *, platform: str, server: str) -> str:
        return (
            "v3-"
            + hashlib.sha256(
                _encoded(self.identity(platform=platform, server=server))
            ).hexdigest()
        )

    def identity(self, *, platform: str, server: str) -> dict[str, Any]:
        binary = self.platforms.get(platform, {}).get(server)
        if binary is None:
            raise ValueError("runtime release does not advertise the selected binary")
        source = {"repository": self.source.repository, "commit": self.source.commit}
        if self.source.tree is not None:
            source["tree"] = self.source.tree
        identity = {
            "schema": RELEASE_SCHEMA,
            "state": self.state,
            "source": source,
            "artifact": self.artifact.__dict__,
            "compatibility": self.compatibility.__dict__,
            "platforms": self.platforms,
            "platform": platform,
            "server": server,
            "binarySha256": binary["sha256"],
        }
        if self.provenance is not None:
            identity["provenance"] = self.provenance
        if self.classes is not None:
            identity["classes"] = self.classes
        return identity

    def marker(
        self, *, platform: str, server: str, binary_sha256: str
    ) -> dict[str, Any]:
        return {
            "schema": "codexy-runtime-marker/v1",
            "identity": self.identity(platform=platform, server=server),
            "installedBinarySha256": binary_sha256,
        }

    def valid_marker(
        self, marker: Any, *, platform: str, server: str, binary: bytes
    ) -> bool:
        return marker == self.marker(
            platform=platform,
            server=server,
            binary_sha256=hashlib.sha256(binary).hexdigest(),
        )

    def package_plugin_root(self) -> str:
        return (
            "codexy-devtools"
            if self.state in {"candidate-proven", "source-selected"}
            else "codexy"
        )

    def verify_archive(self, archive: Path, *, platform: str) -> bool:
        if self.state == "legacy-public":
            return True
        return verify_archive(self, archive, platform=platform)


def load(plugin_root: Path) -> RuntimeRelease:
    path = plugin_root / "runtime-release.json"
    try:
        value = document(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ValueError(f"runtime release is missing or invalid: {error}") from error
    value = object(value, "document")
    state = string(value.get("state"), "state")
    if state not in {"legacy-public", "candidate-proven", "source-selected"}:
        raise ValueError("runtime release state is unsupported")
    source = object(value.get("source"), "source")
    core_aware = "tree" in source or "classes" in value
    source_fields = (
        {"repository", "commit", "tree"} if core_aware else {"repository", "commit"}
    )
    expected = {"schema", "state", "source", "artifact", "compatibility", "platforms"}
    if state == "source-selected":
        expected.add("provenance")
    if core_aware:
        expected.add("classes")
    if set(value) != expected:
        raise ValueError("runtime release has unknown or missing fields")
    if set(source) != source_fields:
        raise ValueError("runtime release source has unknown or missing fields")
    if value.get("schema") != RELEASE_SCHEMA:
        raise ValueError("runtime release schema must be codexy-runtime-release/v1")
    commit = string(source.get("commit"), "source.commit")
    if source.get("repository") != REPOSITORY or not _COMMIT.fullmatch(commit):
        raise ValueError(
            "runtime release source must use the canonical repository and lowercase commit"
        )
    tree = None
    if core_aware:
        tree = string(source.get("tree"), "source.tree")
        if not _COMMIT.fullmatch(tree):
            raise ValueError("runtime release source tree must be lowercase 40-hex")
    artifact = object(value.get("artifact"), "artifact")
    if set(artifact) != {"tag", "url", "sha256", "payloadManifestSha256"}:
        raise ValueError("runtime release artifact has unknown or missing fields")
    tag = string(artifact.get("tag"), "artifact.tag")
    url = string(artifact.get("url"), "artifact.url")
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag):
        raise ValueError("runtime release must use a version-only tag")
    asset = (
        "codexy-runtime-package.tar.gz"
        if state != "legacy-public"
        else "codexy-marketplace-plugin.tar.gz"
    )
    if url != f"{REPOSITORY}/releases/download/{tag}/{asset}":
        raise ValueError("runtime release artifact URL is not canonical")
    parsed_platforms = (
        source_platforms(value.get("platforms"))
        if state == "source-selected"
        else platforms(value.get("platforms"), require_path=state == "candidate-proven")
    )
    provenance = (
        validate_provenance(value.get("provenance"))
        if state == "source-selected"
        else None
    )
    classes = (
        validate_classes(value.get("classes"), parsed_platforms, source)
        if core_aware
        else None
    )
    return RuntimeRelease(
        state,
        Source(REPOSITORY, commit, tree),
        Artifact(
            tag,
            url,
            digest(artifact.get("sha256"), "artifact.sha256"),
            digest(
                artifact.get("payloadManifestSha256"), "artifact.payloadManifestSha256"
            ),
        ),
        compatibility(value.get("compatibility")),
        parsed_platforms,
        provenance,
        classes,
    )
