"""Diagnostic health projection from resolver-admitted observations."""

from .component_diagnostic_surfaces import diagnose_surface
from .component_json import loads
from .component_manifest import ComponentManifest
from .component_resolver import ComponentResolutionError, compare_versions
from .component_source_admission import DiagnosticFailure, DiagnosticTree


def health(manifest: ComponentManifest, actual: tuple[str, ...], recorded: tuple[str, ...] | None, records: dict[str, dict[str, object]], trees: dict[str, DiagnosticTree], admission_error: str | None, marketplace_failure: bool) -> list[dict[str, str]]:
    expected, result = set(recorded or ()) | set(actual), []
    for component in manifest.component_ids:
        if component not in expected:
            continue
        if admission_error or marketplace_failure:
            result.append(_entry(component, "incompatible"))
        elif component not in actual:
            result.append(_entry(component, "missing"))
        elif not _canonical_diagnostics(manifest, component, trees.get(component), records.get(component)):
            result.append(_entry(component, "incompatible"))
        elif _version_relation(manifest, records.get(component)) < 0:
            result.append(_entry(component, "stale"))
        elif _version_relation(manifest, records.get(component)) > 0:
            result.append(_entry(component, "incompatible"))
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


def _canonical_diagnostics(manifest: ComponentManifest, component: str, tree: DiagnosticTree | None, record: dict[str, object] | None) -> bool:
    if tree is None:
        return False
    required = manifest.component(component).asset.required_paths
    if any(tree.read(path).failure for path in required):
        return False
    manifest_ok, failure = _manifest_is_valid(tree, manifest.component(component).plugin, _record_version(record))
    if failure or not manifest_ok:
        return False
    surface = diagnose_surface(tree, component)
    if surface.failure or not surface.canonical:
        return False
    return not _legacy_core_monolith(tree, component)


def _record_version(record: dict[str, object] | None) -> str | None:
    version = record.get("version") if record else None
    return version if isinstance(version, str) else None


def _manifest_is_valid(tree: DiagnosticTree, name: str, version: str | None) -> tuple[bool, DiagnosticFailure | None]:
    read = tree.read(".codex-plugin/plugin.json")
    if read.failure:
        return False, read.failure
    try:
        value = loads(read.contents, object_pairs_hook=_unique_object)  # type: ignore[arg-type]
    except (UnicodeDecodeError, ValueError):
        return False, DiagnosticFailure.MALFORMED
    return (
        isinstance(value, dict)
        and value.get("name") == name
        and value.get("repository") == "https://github.com/eunsoogi/codexy"
        and version is not None
        and value.get("version") == version,
        None,
    )


def _legacy_core_monolith(tree: DiagnosticTree, component: str) -> bool:
    if component != "core":
        return False
    observations = tuple(tree.optional(path) for path in (".mcp.json", ".codex/lsp-client.json", "lsp", "mcp", "runtime-release.json"))
    return any(observation.failure or observation.present for observation in observations)


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("diagnostic JSON has duplicate keys")
        result[key] = value
    return result
