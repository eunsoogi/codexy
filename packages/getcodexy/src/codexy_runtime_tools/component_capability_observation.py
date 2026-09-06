"""Additive capability observations for the read-only doctor report."""

from __future__ import annotations

from typing import Mapping


UNKNOWN = "unknown"
_DIRECT_SOURCE = "getcodexy-direct-probe"
_DIRECT_SCOPE = "plugin-subprocess"
_REGISTRATION_SOURCE = "getcodexy-registration-check"
_REGISTRATION_SCOPE = "current-doctor-invocation"

CAPABILITIES = {
    "core": (
        "hook:codexy-thread-delivery",
        "specialist:codexy-architect",
        "specialist:codexy-auditor",
        "specialist:codexy-cartographer",
        "specialist:codexy-inspector",
        "specialist:codexy-sentinel",
        "specialist:codexy-shipwright",
        "specialist:codexy-warden",
    ),
    "github": ("hook:codexy-github-workflow-context", "specialist:codexy-weaver"),
    "devtools": ("mcp:codegraph", "mcp:lsp"),
}


def component_observations(
    component: str,
    configured: bool,
    probe: Mapping[str, object] | None = None,
) -> dict[str, dict[str, object]]:
    """Return per-capability states without claiming native host execution."""
    observations = {
        name: _entry(configured, source=_REGISTRATION_SOURCE, scope=_REGISTRATION_SCOPE)
        for name in CAPABILITIES.get(component, ())
    }
    if not probe:
        return observations
    direct = probe.get("_capability_probes")
    if not isinstance(direct, dict):
        direct = {}
    for name in observations:
        details = direct.get(name)
        if isinstance(details, dict):
            observations[name] = _entry(
                configured,
                started=details.get("started") is True,
                callable=details.get("callable") is True,
                source=_DIRECT_SOURCE,
                scope=_DIRECT_SCOPE,
            )
    return observations


def probe_observation(
    capability: str,
    configured: bool,
    started: bool,
    callable_: bool,
) -> dict[str, dict[str, object]]:
    """Describe one direct probe for health's additive observation mapping."""
    return {
        capability: {
            "configured": configured,
            "started": started,
            "callable": callable_,
        }
    }


def record_probe(
    target: dict[str, object],
    capability: str,
    configured: bool,
    started: bool,
    callable_: bool,
) -> None:
    """Attach one current direct-probe result to an internal probe payload."""
    probes = target.setdefault("_capability_probes", {})
    if isinstance(probes, dict):
        probes.update(probe_observation(capability, configured, started, callable_))


def _entry(
    configured: bool,
    *,
    started: bool = False,
    callable: bool = False,
    source: str,
    scope: str,
) -> dict[str, object]:
    return {
        "states": {
            "configured": "configured" if configured else UNKNOWN,
            "loaded": "loaded" if started else UNKNOWN,
            "callable": "callable" if callable else UNKNOWN,
            "verified": UNKNOWN,
        },
        "source": source if configured or started or callable else UNKNOWN,
        "scope": scope if configured or started or callable else UNKNOWN,
        "host_id": None,
        "session_id": None,
    }
