"""Validation helpers for authenticated runtime-release payloads."""

from __future__ import annotations

import hashlib
import json
import tarfile
from pathlib import Path
from typing import Any, TYPE_CHECKING

from .identity import (
    CANDIDATE_PLATFORMS,
    PUBLIC_PLATFORMS,
    SERVERS,
    compatibility,
    digest,
    document,
    object,
    platforms,
    string,
)
from .release_provenance import PROVENANCE_WORKFLOW, validate_provenance
from .release_watcher_contract import validate_watcher

if TYPE_CHECKING:
    from .contract import RuntimeRelease

REPOSITORY = "https://github.com/eunsoogi/codexy"
REPOSITORY_ID = 1_269_350_143


def encoded(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def canonical_digest(value: Any) -> str:
    return hashlib.sha256(encoded(value)).hexdigest()


def source_platforms(value: Any) -> dict[str, dict[str, dict[str, str]]]:
    value = object(value, "source platforms")
    if set(value) != PUBLIC_PLATFORMS:
        raise ValueError(
            "source-selected runtime must advertise exactly darwin and linux"
        )
    result: dict[str, dict[str, dict[str, str]]] = {}
    for platform in PUBLIC_PLATFORMS:
        inventory = object(value.get(platform), f"source platforms.{platform}")
        if set(inventory) != SERVERS:
            raise ValueError("source-selected runtime has unknown or missing server")
        binaries: dict[str, dict[str, str]] = {}
        for server in SERVERS:
            item = object(
                inventory.get(server), f"source platforms.{platform}.{server}"
            )
            if set(item) != {"path", "sha256"}:
                raise ValueError(
                    "source-selected runtime binary has unknown or missing fields"
                )
            path = string(item.get("path"), "source binary.path")
            extension = "exe" if platform == "windows-x86_64" else "bin"
            expected = f"runtime/codexy-mcp-{server}-{platform}.{extension}"
            if path != expected or path.casefold() != path:
                raise ValueError("source-selected runtime binary path is not canonical")
            binaries[server] = {
                "path": path,
                "sha256": digest(item.get("sha256"), "source binary.sha256"),
            }
        result[platform] = binaries
    return result


def validate_classes(
    value: Any,
    expected_platforms: dict[str, dict[str, dict[str, str]]],
    source: dict[str, Any],
) -> dict[str, Any]:
    value = object(value, "classes")
    expected_fields = {"devtoolsMcp", "coreHandoff"}
    if "coreWatcherMcp" in value:
        expected_fields.add("coreWatcherMcp")
    if set(value) != expected_fields:
        raise ValueError("runtime release classes have unknown or missing fields")
    devtools = object(value.get("devtoolsMcp"), "devtoolsMcp")
    if (
        set(devtools) != {"platforms"}
        or devtools.get("platforms") != expected_platforms
    ):
        raise ValueError(
            "runtime release devtools class does not bind its platform inventory"
        )
    core = object(value.get("coreHandoff"), "coreHandoff")
    if set(core) != {"manifest", "platforms"}:
        raise ValueError("runtime release core handoff has unknown or missing fields")
    manifest = object(core.get("manifest"), "core manifest")
    if (
        set(manifest) != {"path", "sha256"}
        or manifest.get("path") != "handoff-runtime.json"
    ):
        raise ValueError("runtime release core manifest identity is not canonical")
    digest(manifest.get("sha256"), "core manifest.sha256")
    bridges = object(core.get("platforms"), "core platforms")
    if set(bridges) != CANDIDATE_PLATFORMS:
        raise ValueError(
            "runtime release core handoff must cover all candidate platforms"
        )
    for platform, bridge in bridges.items():
        bridge = object(bridge, f"core platforms.{platform}")
        if set(bridge) != {"path", "sha256", "kind"}:
            raise ValueError(
                "runtime release core bridge has unknown or missing fields"
            )
        extension = "exe" if platform == "windows-x86_64" else "bin"
        if (
            bridge.get("path")
            != f"runtime/codexy-handoff-validate-{platform}.{extension}"
        ):
            raise ValueError("runtime release core bridge path is not canonical")
        digest(bridge.get("sha256"), "core bridge.sha256")
        expected_kind = {
            "darwin-arm64": "mach-o",
            "linux-x86_64": "elf",
            "windows-x86_64": "pe",
        }[platform]
        if bridge.get("kind") != expected_kind:
            raise ValueError("runtime release core bridge kind is not canonical")
    watcher = value.get("coreWatcherMcp")
    if watcher is not None:
        validate_watcher(watcher)
    if source.get("repository") != REPOSITORY or not isinstance(
        source.get("commit"), str
    ):
        raise ValueError(
            "runtime release classes are not bound to the canonical source"
        )
    if "tree" not in source:
        raise ValueError("runtime release classes require a source tree identity")
    return value


def verify_archive(release: "RuntimeRelease", archive: Path, *, platform: str) -> bool:
    with tarfile.open(archive, "r:gz") as package:
        names = [member.name for member in package.getmembers()]
        if len({name.casefold() for name in names}) != len(names):
            raise ValueError("runtime archive has duplicate or casefold paths")
        plugin_root = release.package_plugin_root()
        package.getmember(f"plugins/{plugin_root}/.codex-plugin/plugin.json")
        candidate_file = package.extractfile(
            f"plugins/{plugin_root}/runtime-candidate.json"
        )
        if candidate_file is None:
            raise ValueError("runtime archive is missing its candidate receipt")
        candidate = document(candidate_file.read().decode())
        if canonical_digest(candidate) != release.artifact.payload_manifest_sha256:
            raise ValueError("runtime candidate digest does not match release")
        _validate_candidate(candidate, release, package, platform, plugin_root)
    return True


def _validate_candidate(
    candidate: Any,
    release: "RuntimeRelease",
    package: tarfile.TarFile,
    platform: str,
    plugin_root: str,
) -> None:
    candidate = object(candidate, "candidate")
    expected = {"schema", "source", "artifact", "compatibility", "platforms"}
    if release.state == "source-selected" and release.classes is not None:
        expected.add("classes")
    if (
        release.state not in {"source-selected", "candidate-proven"}
        or set(candidate) != expected
        or candidate.get("schema") != "codexy-runtime-candidate/v1"
    ):
        raise ValueError("runtime candidate schema or state does not match release")
    candidate_source = object(candidate.get("source"), "candidate source")
    release_source = {
        "repository": release.source.repository,
        "commit": release.source.commit,
    }
    if release.source.tree is not None:
        release_source["tree"] = release.source.tree
    if candidate_source != release_source:
        raise ValueError("runtime candidate source identity does not match release")
    artifact = object(candidate.get("artifact"), "candidate artifact")
    if set(artifact) != {"stagingRunId", "stagingRunAttempt"} or not all(
        type(artifact[field]) is int and artifact[field] > 0 for field in artifact
    ):
        raise ValueError("runtime candidate staging identity is invalid")
    if release.provenance is not None and (
        artifact["stagingRunId"] != release.provenance["runId"]
        or artifact["stagingRunAttempt"] != release.provenance["runAttempt"]
    ):
        raise ValueError("runtime candidate staging identity does not match provenance")
    if compatibility(candidate.get("compatibility")) != release.compatibility:
        raise ValueError("runtime candidate compatibility does not match release")
    inventory = platforms(candidate.get("platforms"), require_path=True)
    if release.state == "source-selected":
        if (
            set(inventory) != CANDIDATE_PLATFORMS
            or {platform: inventory[platform] for platform in PUBLIC_PLATFORMS}
            != release.platforms
        ):
            raise ValueError(
                "source-selected runtime inventory does not match candidate"
            )
        if release.classes is not None:
            candidate_classes = validate_classes(
                candidate.get("classes"), inventory, release_source
            )
            if candidate_classes["coreHandoff"] != release.classes["coreHandoff"] or (
                "coreWatcherMcp" in release.classes
                and candidate_classes.get("coreWatcherMcp")
                != release.classes["coreWatcherMcp"]
            ):
                raise ValueError(
                    "source-selected core runtime class identity does not match candidate"
                )
    elif inventory != release.platforms:
        raise ValueError("runtime candidate inventory does not match release")
    if platform not in inventory:
        raise ValueError("runtime candidate does not include the selected platform")
    for candidate_platform, binaries in inventory.items():
        for server in SERVERS:
            binary = binaries[server]
            member = package.extractfile(f"plugins/{plugin_root}/{binary['path']}")
            if (
                member is None
                or hashlib.sha256(member.read()).hexdigest() != binary["sha256"]
            ):
                raise ValueError(
                    f"runtime candidate {candidate_platform}/{server} digest does not match"
                )
    if release.classes and "coreWatcherMcp" in release.classes:
        watcher = release.classes["coreWatcherMcp"]["platforms"]
        for candidate_platform, binary in watcher.items():
            member = package.extractfile(f"plugins/{plugin_root}/{binary['path']}")
            if (
                member is None
                or hashlib.sha256(member.read()).hexdigest() != binary["sha256"]
            ):
                raise ValueError(
                    f"runtime candidate {candidate_platform}/watcher digest does not match"
                )
