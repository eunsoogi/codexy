"""Installed-component health classification for read-only inspection."""

from __future__ import annotations

from .component_capability_probe import (
    FAILURES,
    identity_matches as _identity_matches,
    probe_component as _probe_component,
    probe_reason as _probe_reason,
)
from .component_hook_activation import ACTIVATION_STATES
from .component_health_support import (
    _authority_valid,
    _legacy_state,
    _observed,
    _plugin_root,
    manifest_is_valid,
    record_version,
    version_relation,
)
from .component_manifest import ComponentManifest
from .component_registration_health import valid_registration


def health(
    manifest: ComponentManifest,
    actual: tuple[str, ...],
    recorded: tuple[str, ...] | None,
    records: dict[str, dict[str, object]],
    admission_error: str | None,
    host_error: bool,
    activation: dict[str, str] | None = None,
) -> list[dict[str, object]]:
    expected = set(recorded or ()) | set(actual)
    return [
        _component_health(
            manifest,
            component,
            actual,
            records,
            admission_error,
            host_error,
            activation,
        )
        for component in manifest.component_ids
        if component in expected
    ]


def _component_health(
    manifest,
    component,
    actual,
    records,
    admission_error,
    host_error,
    activation,
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
    if isinstance(probe.get("_capability_probe"), dict):
        result["observed"]["capability_probe"] = dict(
            probe["_capability_probe"]
        )
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
    fallback, restart = FAILURES[reason]
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
