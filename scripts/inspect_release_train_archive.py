#!/usr/bin/env python3
"""Fail closed on a complete, deterministic Codexy marketplace release train."""

import hashlib
import json
import re
import sys
import tarfile
import unicodedata
from pathlib import Path

ARCHIVE, CHECKOUT, TAG = map(Path, sys.argv[1:4])
target = str(TAG).removeprefix("v")
component_path = (
    CHECKOUT / "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"
)
marketplace_path = CHECKOUT / ".agents/plugins/marketplace.json"
components = json.loads(component_path.read_text())["components"]
marketplace = json.loads(marketplace_path.read_text())
activation = json.loads(
    (CHECKOUT / ".agents/plugins/runtime-activation.json").read_text()
)["candidate"]
runtime_platforms = list(activation["platforms"])
core_handoff = activation.get("classes", {}).get("coreHandoff")
inventory = [
    (item["id"], item["plugin"], item["asset"]["packageRoot"]) for item in components
]
devices = {
    "con",
    "prn",
    "aux",
    "nul",
    *(f"com{i}" for i in range(1, 10)),
    *(f"lpt{i}" for i in range(1, 10)),
}
if inventory != [
    ("core", "codexy", "plugins/codexy"),
    ("github", "codexy-github", "plugins/codexy-github"),
    ("devtools", "codexy-devtools", "plugins/codexy-devtools"),
]:
    raise SystemExit("unsupported release-train component inventory")
if any(item["version"] != target for item in components):
    raise SystemExit("component manifest version mismatch")
if any(item["version"] != target for item in marketplace["plugins"]):
    raise SystemExit("marketplace version mismatch")
if [
    (item["name"], item["source"]["path"].removeprefix("./"))
    for item in marketplace["plugins"]
] != [(plugin, root) for _, plugin, root in inventory]:
    raise SystemExit("marketplace paths mismatch")


def reject(message: str) -> None:
    raise SystemExit(message)


def materialized_source_bytes(plugin: str, relative: str, source_bytes: bytes) -> bytes:
    if plugin != "codexy-devtools":
        return source_bytes
    if relative == ".codex-plugin/plugin.json":
        source_manifest = json.loads(source_bytes)
        source_manifest["supportedPlatforms"] = runtime_platforms
        return (json.dumps(source_manifest, indent=2) + "\n").encode()
    if relative != "mcp/codexy-mcp-devtools":
        return source_bytes
    text = source_bytes.decode()
    source_declaration = 'bundled_platforms="darwin-arm64 linux-x86_64"'
    target_declaration = 'bundled_platforms="' + " ".join(runtime_platforms) + '"'
    if text.count(source_declaration) != 1:
        reject("activation checkout wrapper platform declaration mismatch")
    text = text.replace(source_declaration, target_declaration, 1)
    text, replacements = re.subn(
        r"(?<=exec uvx --from getcodexy==)\d+\.\d+\.\d+", target, text
    )
    if replacements != 1:
        reject("activation checkout wrapper version declaration mismatch")
    return text.encode()


entries: dict[str, bytes] = {}
directories: set[str] = set()
identities: set[str] = set()
total_bytes = 0
with tarfile.open(ARCHIVE, "r:gz") as archive:
    for count, member in enumerate(archive, start=1):
        name = member.name
        if count > 10_000:
            reject("archive contains too many entries")
        pieces = tuple(part for part in name.split("/") if part != ".")
        if (
            not (member.isfile() or member.isdir())
            or not pieces
            or name != "/".join(pieces)
        ):
            reject(f"unsafe archive entry: {name}")
        if "\\" in name or ":" in name or name.startswith("/") or ".." in pieces:
            reject(f"unsafe archive path: {name}")
        identity = unicodedata.normalize("NFC", name).casefold()
        if identity in identities:
            reject(f"colliding archive path: {name}")
        identities.add(identity)
        for piece in pieces:
            folded = unicodedata.normalize("NFC", piece).casefold()
            if folded.rstrip(" .") != folded or folded.split(".", 1)[0] in devices:
                reject(f"unsafe Windows archive path: {name}")
        if member.isfile():
            if member.size > 52_428_800:
                reject(f"oversized archive entry: {name}")
            total_bytes += member.size
            if total_bytes > 268_435_456:
                reject("archive uncompressed size exceeds the configured limit")
            extracted = archive.extractfile(member)
            entries[name] = extracted.read() if extracted else b""
        else:
            directories.add(name)

if ".agents/plugins/marketplace.json" not in entries:
    reject("bundle marketplace metadata missing")
if entries[".agents/plugins/marketplace.json"] != marketplace_path.read_bytes():
    reject("bundle marketplace metadata differs from activation checkout")
expected_entries = {".agents/plugins/marketplace.json"}
expected_directories = {".agents", ".agents/plugins"}


def admit_handoff(prefix: str, label: str) -> None:
    if not core_handoff:
        return
    manifest_name = f"{prefix}handoff-runtime.json"
    manifest_bytes = entries.get(manifest_name, b"")
    handoff = json.loads(manifest_bytes or b"null")
    if (
        handoff.get("source")
        != {
            "commit": activation["source"]["commit"],
            "tree": activation["source"]["tree"],
        }
        or hashlib.sha256(manifest_bytes).hexdigest()
        != core_handoff["manifest"]["sha256"]
        or handoff.get("platforms") != core_handoff["platforms"]
    ):
        reject(f"{label} handoff manifest differs from activated class identity")
    expected_entries.add(manifest_name)
    for binary in core_handoff["platforms"].values():
        name = f"{prefix}{binary['path']}"
        if hashlib.sha256(entries.get(name, b"")).hexdigest() != binary["sha256"]:
            reject(f"{label} handoff binary differs from activated class identity")
        expected_entries.add(name)


for _, plugin, package_root in inventory:
    prefix = f"{package_root}/"
    expected_directories.add(package_root)
    manifest_name = f"{prefix}.codex-plugin/plugin.json"
    if manifest_name not in entries:
        reject(f"component manifest missing: {plugin}")
    manifest = json.loads(entries[manifest_name])
    if manifest.get("name") != plugin or manifest.get("version") != target:
        reject(f"component manifest mismatch: {plugin}")
    if (
        plugin == "codexy-devtools"
        and manifest.get("supportedPlatforms") != runtime_platforms
    ):
        reject("public devtools manifest does not match activated runtime platforms")
    component = next(item for item in components if item["plugin"] == plugin)
    for required in component["asset"]["requiredPaths"]:
        if f"{prefix}{required}" not in entries:
            reject(f"component required path missing: {plugin}/{required}")
    source = CHECKOUT / package_root
    for path in source.rglob("*"):
        relative = path.relative_to(source).as_posix()
        if path.is_symlink():
            reject(f"activation checkout contains a symlink: {package_root}/{relative}")
        if path.is_dir():
            expected_directories.add(f"{package_root}/{relative}")
            continue
        if not path.is_file():
            continue
        if plugin == "codexy-devtools" and relative.startswith("runtime/"):
            continue
        if relative in {"runtime-candidate.json", "runtime-release.json"}:
            if f"{prefix}{relative}" in entries:
                reject(f"runtime contract leaked: {relative}")
            continue
        expected_entries.add(f"{prefix}{relative}")
        source_bytes = materialized_source_bytes(plugin, relative, path.read_bytes())
        if entries.get(f"{prefix}{relative}") != source_bytes:
            reject(f"component source mismatch: {plugin}/{relative}")
    for name, content in entries.items():
        if name.startswith(prefix) and not name.startswith(f"{prefix}runtime/"):
            if re.search(
                rb"BEGIN(?: [A-Z0-9]+)* PRIVATE KEY|(?:AKIA|ASIA)[0-9A-Z]{16}|(?:/Users|/home|/tmp|/private/var)/",
                content,
            ):
                reject(f"unsafe archive content: {name}")
    if plugin == "codexy" and core_handoff:
        admit_handoff(prefix, "core-owned")
        expected_directories.add(f"{package_root}/runtime")
    if plugin == "codexy-devtools":
        admit_handoff(prefix, "devtools")
        for platform in runtime_platforms:
            extension = "exe" if platform == "windows-x86_64" else "bin"
            for server in ("lsp", "codegraph"):
                expected_entries.add(
                    f"{prefix}runtime/codexy-mcp-{server}-{platform}.{extension}"
                )
        expected_entries.add(f"{prefix}mcp/codexy-mcp-devtools.exe")
        expected_directories.add(f"{package_root}/runtime")
if set(entries) != expected_entries:
    missing = sorted(expected_entries - set(entries))
    extra = sorted(set(entries) - expected_entries)
    reject(
        f"bundle files differ from the complete release-train inventory: missing={missing} extra={extra}"
    )
if directories != expected_directories:
    reject("bundle directories differ from the complete release-train inventory")
print(f"release_train_sha256 {hashlib.sha256(ARCHIVE.read_bytes()).hexdigest()}")
