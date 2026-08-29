"""Installed-component health classification for read-only inspection."""

from __future__ import annotations

import json
import os
from pathlib import Path

from .component_capability_probe import (
    FAILURES,
    identity_matches as _identity_matches,
    probe_component as _probe_component,
    probe_reason as _probe_reason,
)
from .component_manifest import ComponentManifest
from .component_registration_health import valid_registration
from .component_resolver import ComponentResolutionError, compare_versions


SURFACE_PATHS = {
    "core": ("agents/catalog.toml", "hooks/hooks.json", "skills/wiki/SKILL.md"),
    "github": ("agents/catalog.toml", "hooks/hooks.json"),
    "devtools": ("mcp/codexy-mcp-devtools", ".mcp.json"),
}
AUTHORITY_KEYS = ("authority", "artifact_authority", "artifactAuthority")


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
        _component_health(
            manifest, component, actual, records, admission_error, host_error
        )
        for component in manifest.component_ids
        if component in expected
    ]


def _component_health(
    manifest, component, actual, records, admission_error, host_error
):
    record = records.get(component)
    installed = component in actual
    plugin = _plugin_root(record)
    configured = bool(
        installed
        and plugin
        and manifest_is_valid(
            plugin, manifest.component(component).plugin, record_version(record)
        )
        and valid_registration(plugin, component)
    )
    result = dict(
        component=component,
        state=_legacy_state(
            manifest, component, actual, records, admission_error, host_error
        ),
        installed=installed,
        configured=configured,
        started=False,
        callable=False,
        healthy=False,
        first_failure_stage=None,
        reason_code=None,
        safe_fallback=None,
        restart_required=False,
        observed=_observed(record),
    )
    checks = (
        (admission_error or host_error, "installed", "trusted-inventory-unavailable"),
        (not installed, "installed", "component-not-installed"),
        (not configured, "configured", "component-not-configured"),
    )
    for failed, stage, reason in checks:
        if failed:
            return _mark(result, stage, reason)
    probe = _probe_component(component, plugin, record)
    result["started"], result["callable"] = (
        bool(probe.get("started")),
        bool(probe.get("callable")),
    )
    result["observed"]["runtime"] = {
        "name": probe.get("runtime_name"),
        "version": probe.get("runtime_version"),
    }
    for ready, stage, default in (
        (result["started"], "started", "component-start-failed"),
        (result["callable"], "callable", "capability-call-failed"),
    ):
        if not ready:
            return _mark(result, stage, _probe_reason(probe, default))
    if (
        not _identity_matches(manifest, component, record, probe)
        or version_relation(manifest, record) != 0
    ):
        return _mark(result, "identity", "runtime-identity-mismatch")
    if not _authority_valid(record):
        return _mark(result, "authority", "artifact-authority-invalid")
    result["healthy"] = True
    return result


def _legacy_state(manifest, component, actual, records, admission_error, host_error):
    record = records.get(component)
    if admission_error or host_error:
        return "incompatible"
    if component not in actual:
        return "missing"
    relation = version_relation(manifest, record)
    if relation != 0:
        return "stale" if relation < 0 else "incompatible"
    plugin = _plugin_root(record)
    if plugin is None or not _required_files(manifest, component, plugin):
        return "stale"
    if not manifest_is_valid(
        plugin, manifest.component(component).plugin, record_version(record)
    ) or not valid_registration(plugin, component):
        return "incompatible"
    if component == "devtools" and not os.access(
        plugin / "mcp/codexy-mcp-devtools", os.X_OK
    ):
        return "stale"
    if component == "core" and any(
        os.path.lexists(plugin / path)
        for path in (
            ".mcp.json",
            ".codex/lsp-client.json",
            "lsp",
            "mcp",
            "runtime-release.json",
        )
    ):
        return "stale"
    if not set(manifest.component(component).dependencies).issubset(actual):
        return "incompatible"
    return "healthy"


def _mark(result: dict[str, object], stage: str, reason: str) -> dict[str, object]:
    fallback, restart = FAILURES[reason]
    result.update(
        first_failure_stage=stage,
        reason_code=reason,
        safe_fallback=fallback,
        restart_required=restart,
        repair=fallback,
    )
    if stage in {"started", "callable", "authority"} or (
        stage == "identity" and result["state"] == "healthy"
    ):
        result["state"] = "incompatible"
    return result


def _required_files(manifest, component, plugin):
    paths = (
        manifest.component(component).asset.required_paths + SURFACE_PATHS[component]
    )
    return all(
        (plugin / path).is_file() and not (plugin / path).is_symlink() for path in paths
    )


def _plugin_root(record):
    source = (record or {}).get("source")
    value = source.get("path") if isinstance(source, dict) else None
    return Path(value) if isinstance(value, str) and Path(value).is_absolute() else None


def _observed(record):
    return dict(
        plugin=dict(name=(record or {}).get("name"), version=record_version(record)),
        runtime=dict(name=None, version=None),
    )


def _authority_valid(record):
    if not record:
        return False
    key = next((key for key in AUTHORITY_KEYS if key in record), None)
    if key is None:
        if record.get("marketplaceSource") == {
            "sourceType": "git",
            "source": "https://github.com/eunsoogi/codexy.git",
        }:
            return True
        installed_source = record.get("source")
        marketplace_source = record.get("marketplaceSource")
        if not (
            isinstance(installed_source, dict)
            and isinstance(installed_source.get("path"), str)
            and isinstance(marketplace_source, dict)
            and marketplace_source.get("sourceType") == "local"
            and isinstance(marketplace_source.get("source"), str)
        ):
            return False
        path, root = Path(installed_source["path"]), Path(marketplace_source["source"])
        return (
            path.is_absolute()
            and root.is_absolute()
            and path.parent.name == "plugins"
            and path.parent.parent == root
        )
    authority = record[key]
    return isinstance(authority, dict) and authority.get("state") in {
        "valid",
        "attested",
    }


def version_relation(manifest, record):
    try:
        return compare_versions(record_version(record), manifest.version)
    except ComponentResolutionError:
        return 1


def record_version(record):
    value = record.get("version") if record else None
    return value if isinstance(value, str) else None


def manifest_is_valid(plugin, name, version):
    value = json_value(plugin / ".codex-plugin/plugin.json")
    return (
        version is not None
        and isinstance(value, dict)
        and (value.get("name"), value.get("repository"), value.get("version"))
        == (name, "https://github.com/eunsoogi/codexy", version)
    )


def json_value(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError, json.JSONDecodeError):
        return None
