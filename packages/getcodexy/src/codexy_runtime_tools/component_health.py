"""Installed-component health classification for read-only inspection."""

from __future__ import annotations

from . import component_capability_probe as _probe
from .component_capability_observation import component_observations
from .component_hook_activation import ACTIVATION_STATES
from .component_mcp_materialization import (
    MCP_COMPONENTS,
    mcp_spec_for_plugin,
    valid_component_mcp_cache,
)
from .component_health_support import (
    _authority_valid,
    _health_plugin,
    _legacy_state,
    _observed,
    _plugin_root,
    _selected_registration_valid,
    manifest_is_valid,
    record_version,
    version_relation,
)
from .component_manifest import ComponentManifest
from .updater import compare_managed_files

FAILURES = _probe.FAILURES
_identity_matches = _probe.identity_matches
_probe_component = _probe.probe_component
_probe_reason = _probe.probe_reason


def health(
    manifest: ComponentManifest,
    actual: tuple[str, ...],
    recorded: tuple[str, ...] | None,
    records: dict[str, dict[str, object]],
    admission_error: str | None,
    host_error: bool,
    activation: dict[str, str] | None = None,
    codex_home=None,
) -> list[dict[str, object]]:
    expected = set(recorded or ()) | set(actual)  # Include known and live components.
    context = (
        manifest,
        actual,
        records,
        admission_error,
        host_error,
        activation,
        codex_home,
    )
    return [
        _component_health(context, component)
        for component in manifest.component_ids
        if component in expected
    ]


def _component_health(context, component):
    manifest, actual, records, admission_error, host_error, activation, codex_home = (
        context
    )
    record = records.get(component)
    installed = component in actual
    expected_mcp_spec = None
    if component in MCP_COMPONENTS:
        source_plugin = _plugin_root(record)
        if source_plugin is not None:
            try:
                # Check package shape separately; authority remains a later health check.
                expected_mcp_spec = mcp_spec_for_plugin(
                    source_plugin, component, record_version(record)
                )
            except (OSError, RuntimeError, ValueError):
                expected_mcp_spec = None
    plugin = _health_plugin(manifest, component, record, codex_home)
    registration = (
        compare_managed_files(plugin, codex_home, component)
        if plugin and codex_home
        else None
    )
    configured = bool(
        installed
        and plugin
        and manifest_is_valid(
            plugin, manifest.component(component).plugin, record_version(record)
        )
        and _selected_registration_valid(
            plugin, component, record_version(record), expected_mcp_spec
        )
        and (
            not registration
            or not registration["observed"]
            or registration["state"] == "exact"
        )
        and (
            component not in MCP_COMPONENTS
            or (
                expected_mcp_spec is not None
                and valid_component_mcp_cache(
                    codex_home,
                    component,
                    manifest.version,
                    expected_spec=expected_mcp_spec,
                )
            )
        )
    )
    state = _legacy_state(
        manifest,
        component,
        actual,
        records,
        admission_error,
        host_error,
        codex_home,
        expected_mcp_spec,
    )
    registration_state = registration["state"] if registration else "exact"
    if state in {"healthy", "stale"} and registration_state != "exact":
        state = {"unmanaged-conflict": "incompatible", "missing": "missing"}.get(
            registration_state, "stale"
        )
    result = dict(
        component=component,
        state=state,
        installed=installed,
        configured=configured,
        started=False,
        callable=False,
        healthy=False,
        first_failure_stage=None,
        reason_code=None,
        safe_fallback=None,
        restart_required=False,
        observed={
            **_observed(record),
            # Keep a no-MCP core accurate when activation prevents its live probe.
            "capabilities": component_observations(
                component,
                configured,
                {
                    "_capability_contract": {
                        "mcp:watcher": "watcher" in expected_mcp_spec.servers
                    }
                }
                if component == "core" and expected_mcp_spec is not None
                else None,
            ),
            "registration": registration,
        },
    )
    checks = (
        (admission_error or host_error, "installed", "trusted-inventory-unavailable"),
        (not installed, "installed", "component-not-installed"),
        (not configured, "configured", "component-not-configured"),
    )
    for failed, stage, reason in checks:  # Report the earliest failure.
        if failed:
            return _mark(result, stage, reason)
    if activation is not None and component in activation:
        return _mark(result, "activation", activation[component])
    probe = _probe_component(component, plugin, record)
    result["started"], result["callable"] = (
        bool(probe.get("started")),
        bool(probe.get("callable")),
    )
    result["observed"]["runtime"] = {
        "name": probe.get("runtime_name"),
        "version": probe.get("runtime_version"),
    }
    result["observed"]["capabilities"] = component_observations(
        component, configured, probe
    )
    if isinstance(probe.get("_capability_probe"), dict):
        result["observed"]["capability_probe"] = dict(probe["_capability_probe"])
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


def _mark(result: dict[str, object], stage: str, reason: str) -> dict[str, object]:
    fallback, restart = FAILURES[reason]  # Map the reason to recovery guidance.
    result.update(
        first_failure_stage=stage,
        reason_code=reason,
        safe_fallback=fallback,
        restart_required=restart,
        repair=fallback,
    )
    if stage == "activation":
        result["state"] = ACTIVATION_STATES.get(reason, "incompatible")
    elif stage in {"started", "callable", "authority"} or (
        stage == "identity" and result["state"] == "healthy"
    ):
        result["state"] = "incompatible"
    return result
