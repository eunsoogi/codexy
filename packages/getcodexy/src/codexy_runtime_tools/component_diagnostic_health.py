"""Diagnostic health projection from resolver-admitted observations."""

import json

from .component_diagnostic_surfaces import valid_surface
from .component_manifest import ComponentManifest
from .component_resolver import ComponentResolutionError, compare_versions
from .component_source_admission import DiagnosticTree


def health(manifest: ComponentManifest, actual: tuple[str, ...], recorded: tuple[str, ...] | None, records: dict[str, dict[str, object]], trees: dict[str, DiagnosticTree], admission_error: str | None, marketplace_failure: bool) -> list[dict[str, str]]:
    expected, result = set(recorded or ()) | set(actual), []
    for component in manifest.component_ids:
        if component not in expected:
            continue
        if admission_error or marketplace_failure:
            result.append(_entry(component, "incompatible"))
        elif component not in actual:
            result.append(_entry(component, "missing"))
        elif _version_relation(manifest, records.get(component)) < 0:
            result.append(_entry(component, "stale"))
        elif _version_relation(manifest, records.get(component)) > 0:
            result.append(_entry(component, "incompatible"))
        elif _stale(manifest, component, trees.get(component)):
            result.append(_entry(component, "stale"))
        else:
            result.append({"component": component, "state": "healthy"})
    return result


def _entry(component: str, state: str) -> dict[str, str]:
    repair = "getcodexy bootstrap" if state in {"missing", "stale"} else "repair the Codexy registration, then rerun getcodexy doctor"
    return {"component": component, "state": state, "repair": repair}


def _version_relation(manifest: ComponentManifest, record: dict[str, object] | None) -> int:
    version = record.get("version") if record is not None else None
    if not isinstance(version, str):
        return -1
    try:
        return compare_versions(version, manifest.version)
    except ComponentResolutionError:
        return 1


def _stale(manifest: ComponentManifest, component: str, tree: DiagnosticTree | None) -> bool:
    if tree is None:
        return True
    required = manifest.component(component).asset.required_paths
    if any(tree.read_regular(path) is None for path in required):
        return True
    return not _manifest_is_valid(tree, manifest.component(component).plugin, manifest.version) or not valid_surface(tree, component) or _legacy_core_monolith(tree, component)


def _manifest_is_valid(tree: DiagnosticTree, name: str, version: str) -> bool:
    contents = tree.read_regular(".codex-plugin/plugin.json")
    try:
        value = json.loads(contents.decode()) if contents is not None else None
    except (UnicodeDecodeError, ValueError):
        return False
    return isinstance(value, dict) and value.get("name") == name and value.get("repository") == "https://github.com/eunsoogi/codexy" and value.get("version") == version


def _legacy_core_monolith(tree: DiagnosticTree, component: str) -> bool:
    return component == "core" and any(tree.present_or_unsafe(path) for path in (".mcp.json", ".codex/lsp-client.json", "lsp", "mcp", "runtime-release.json"))
