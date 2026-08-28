"""Installed-component health classification for read-only inspection."""

from __future__ import annotations

import json
import os
from pathlib import Path

from .component_manifest import ComponentManifest
from .component_registration_health import valid_registration
from .component_resolver import ComponentResolutionError, compare_versions


SURFACE_PATHS = {
    "core": ("agents/catalog.toml", "hooks/hooks.json", "skills/wiki/SKILL.md"),
    "github": ("agents/catalog.toml", "hooks/hooks.json"),
    "devtools": ("mcp/codexy-mcp-devtools", ".mcp.json"),
}
FAILURES = {
    "trusted-inventory-unavailable": ("repair installed component inventory, then rerun getcodexy doctor", False),
    "component-not-installed": ("getcodexy bootstrap", True),
    "component-not-configured": ("repair the Codexy registration, then rerun getcodexy doctor", True),
    "component-start-failed": ("repair the installed launcher/runtime, then rerun getcodexy doctor", True),
    "capability-not-exposed": ("repair the Codexy registration, then restart Codex", True),
    "capability-call-failed": ("use the reported safe component fallback and rerun getcodexy doctor", False),
    "runtime-identity-mismatch": ("reinstall the selected release, then restart Codex", True),
    "artifact-authority-invalid": ("reinstall from a trusted release artifact", True),
}


def health(
    manifest: ComponentManifest,
    actual: tuple[str, ...],
    recorded: tuple[str, ...] | None,
    records: dict[str, dict[str, object]],
    admission_error: str | None,
    host_error: bool,
) -> list[dict[str, object]]:
    expected = set(recorded or ()) | set(actual)
    return [
        _component_health(manifest, component, actual, records, admission_error, host_error)
        for component in manifest.component_ids
        if component in expected
    ]
def _component_health(
    manifest: ComponentManifest,
    component: str,
    actual: tuple[str, ...],
    records: dict[str, dict[str, object]],
    admission_error: str | None,
    host_error: bool,
) -> dict[str, object]:
    record = records.get(component)
    installed = component in actual
    configured = installed and _configured(manifest, component, record)
    result = {
        "component": component,
        "state": _legacy_state(manifest, component, actual, records, admission_error, host_error),
        "installed": installed,
        "configured": configured,
        "started": False,
        "callable": False,
        "healthy": False,
        "first_failure_stage": None,
        "reason_code": None,
        "safe_fallback": None,
        "restart_required": False,
        "observed": _observed(record),
    }
    if admission_error or host_error:
        return _mark(result, "installed", "trusted-inventory-unavailable")
    if not installed:
        return _mark(result, "installed", "component-not-installed")
    if not configured:
        return _mark(result, "configured", "component-not-configured")
    if version_relation(manifest, record) != 0:
        return _mark(result, "identity", "runtime-identity-mismatch")
    if not _authority_valid(record):
        return _mark(result, "authority", "artifact-authority-invalid")
    probe = _probe_component(component, _plugin_root(record), record)
    result["started"], result["callable"] = bool(probe.get("started")), bool(probe.get("callable"))
    result["observed"]["runtime"] = {"name": probe.get("runtime_name"), "version": probe.get("runtime_version")}
    if not result["started"]:
        return _mark(result, "started", _probe_reason(probe, "component-start-failed"))
    if not result["callable"]:
        return _mark(result, "callable", _probe_reason(probe, "capability-call-failed"))
    if not _identity_matches(manifest, component, record, probe):
        return _mark(result, "identity", "runtime-identity-mismatch")
    result["healthy"] = True
    return result


def _legacy_state(
    manifest: ComponentManifest,
    component: str,
    actual: tuple[str, ...],
    records: dict[str, dict[str, object]],
    admission_error: str | None,
    host_error: bool,
) -> str:
    record = records.get(component)
    if admission_error or host_error:
        return "incompatible"
    if component not in actual:
        return "missing"
    relation = version_relation(manifest, record)
    if relation < 0:
        return "stale"
    if relation > 0 or corrupt(manifest, component, record):
        return "incompatible"
    if stale(manifest, component, record):
        return "stale"
    if not set(manifest.component(component).dependencies).issubset(actual):
        return "incompatible"
    return "healthy"
def _mark(result: dict[str, object], stage: str, reason: str) -> dict[str, object]:
    fallback, restart = FAILURES[reason]
    result.update(first_failure_stage=stage, reason_code=reason, safe_fallback=fallback, restart_required=restart, repair=fallback)
    if stage in {"started", "callable", "authority"} or (
        stage == "identity" and result["state"] == "healthy"
    ):
        result["state"] = "incompatible"
    return result
def _probe_reason(probe: dict[str, object], default: str) -> str:
    reason = probe.get("reason_code")
    return reason if isinstance(reason, str) and reason in FAILURES else default


def _configured(
    manifest: ComponentManifest, component: str, record: dict[str, object] | None
) -> bool:
    plugin = _plugin_root(record)
    if plugin is None:
        return False
    required = manifest.component(component).asset.required_paths + SURFACE_PATHS[component]
    return all(regular(plugin / path) for path in required) and manifest_is_valid(
        plugin, manifest.component(component).plugin, record_version(record)
    ) and valid_registration(plugin, component)


def _plugin_root(record: dict[str, object] | None) -> Path | None:
    source = record.get("source") if record else None
    value = source.get("path") if isinstance(source, dict) else None
    return Path(value) if isinstance(value, str) and Path(value).is_absolute() else None


def _observed(record: dict[str, object] | None) -> dict[str, object]:
    return {"plugin": {"name": record.get("name") if record else None, "version": record_version(record)}, "runtime": {"name": None, "version": None}}


def _authority_valid(record: dict[str, object] | None) -> bool:
    if not record:
        return False
    for key in ("authority", "artifact_authority", "artifactAuthority"):
        if key in record:
            value = record[key]
            return isinstance(value, dict) and value.get("state") in {"valid", "attested"}
    return record.get("marketplaceSource") == {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}


def _identity_matches(
    manifest: ComponentManifest,
    component: str,
    record: dict[str, object] | None,
    probe: dict[str, object],
) -> bool:
    names = probe.get("runtime_names")
    versions = probe.get("runtime_versions")
    return (
        record is not None
        and record.get("name") == manifest.component(component).plugin
        and probe.get("runtime_version") == manifest.version
        and (not names or set(names) == {"codexy-codegraph", "codexy-lsp"})
        and (not versions or set(versions) == {manifest.version})
    )
def entry(component: str, state: str) -> dict[str, str]:
    return {"component": component, "state": state, "repair": "getcodexy bootstrap" if state in {"missing", "stale"} else "repair the Codexy registration, then rerun getcodexy doctor"}


def version_relation(
    manifest: ComponentManifest, record: dict[str, object] | None
) -> int:
    version = record.get("version") if record else None
    try:
        return compare_versions(version, manifest.version) if isinstance(version, str) else 1
    except ComponentResolutionError:
        return 1


def stale(
    manifest: ComponentManifest, component: str, record: dict[str, object] | None
) -> bool:
    plugin = _plugin_root(record)
    if plugin is None:
        return True
    required = manifest.component(component).asset.required_paths + SURFACE_PATHS[component]
    if any(not regular(plugin / path) for path in required):
        return True
    if component == "devtools" and not os.access(plugin / "mcp/codexy-mcp-devtools", os.X_OK):
        return True
    return has_legacy_core_monolith(plugin, component)


def corrupt(
    manifest: ComponentManifest, component: str, record: dict[str, object] | None
) -> bool:
    plugin = _plugin_root(record)
    if plugin is None:
        return False
    required = manifest.component(component).asset.required_paths + SURFACE_PATHS[component]
    return all(regular(plugin / path) for path in required) and (
        not manifest_is_valid(plugin, manifest.component(component).plugin, record_version(record))
        or not valid_registration(plugin, component)
    )


def record_version(record: dict[str, object] | None) -> str | None:
    value = record.get("version") if record else None
    return value if isinstance(value, str) else None


def regular(path: Path) -> bool:
    try:
        return path.is_file() and not path.is_symlink()
    except OSError:
        return False


def manifest_is_valid(plugin: Path, name: str, version: str | None) -> bool:
    value = json_value(plugin / ".codex-plugin/plugin.json")
    return isinstance(value, dict) and value.get("name") == name and value.get("repository") == "https://github.com/eunsoogi/codexy" and version is not None and value.get("version") == version


def has_legacy_core_monolith(plugin: Path, component: str) -> bool:
    return component == "core" and any(os.path.lexists(plugin / path) for path in (".mcp.json", ".codex/lsp-client.json", "lsp", "mcp", "runtime-release.json"))


def json_value(path: Path) -> object | None:
    try:
        with path.open("r", encoding="utf-8") as source:
            return json.load(source)
    except (OSError, UnicodeDecodeError, ValueError, json.JSONDecodeError):
        return None


def _probe_component(component: str, plugin: Path | None, record: dict[str, object]) -> dict[str, object]:
    from .component_cli import _probe_component as live_probe
    return live_probe(component, plugin, record)
