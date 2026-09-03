"""Classify effective host hook rows against installed plugin registrations."""

from __future__ import annotations

import json
from pathlib import Path

from .component_hook_activation_expected import ExpectedHook
from .component_hook_activation_host import HookStateError, _rows


def classify_activation(
    expected: dict[str, tuple[ExpectedHook, ...]], rows: object
) -> dict[str, str]:
    observed = _rows(rows)
    by_key: dict[str, list[dict[str, object]]] = {}
    for row in observed:
        key = row.get("key")
        if isinstance(key, str):
            by_key.setdefault(key, []).append(row)

    failures: dict[str, str] = {}
    for component, required in expected.items():
        if not required:
            failures[component] = "required-hook-trust-missing"
            continue
        required_keys = {entry.key for entry in required}
        component_rows = [
            row
            for row in observed
            if row.get("pluginId") == required[0].plugin_id
            or row.get("sourcePath") == str(required[0].source_path)
        ]
        component_keys = {
            row.get("key") for row in component_rows if isinstance(row.get("key"), str)
        }
        if component_keys - required_keys:
            failures[component] = "required-hook-trust-stale"
            continue
        for entry in required:
            matches = by_key.get(entry.key, [])
            if not matches:
                failures[component] = "required-hook-trust-missing"
                break
            if len(matches) != 1:
                failures[component] = "required-hook-trust-stale"
                break
            row = matches[0]
            if row.get("enabled") is not True:
                failures[component] = "required-hook-disabled"
                break
            trust = row.get("trustStatus")
            if trust in {"modified", "stale"}:
                failures[component] = "required-hook-trust-stale"
                break
            if trust != "trusted" and not (
                trust == "managed" and row.get("isManaged") is True
            ):
                failures[component] = "required-hook-trust-missing"
                break
            if not _identity_matches(entry, row):
                failures[component] = "required-hook-trust-stale"
                break
    return failures


def _identity_matches(entry: ExpectedHook, row: dict[str, object]) -> bool:
    return (
        row.get("key") == entry.key
        and row.get("pluginId") == entry.plugin_id
        and _source_matches(entry, row)
        and row.get("eventName") == entry.event_name
        and row.get("handlerType") == "command"
        and row.get("matcher") == entry.matcher
        and _command_matches(entry, row)
        and row.get("timeoutSec") == entry.timeout
        and row.get("async") is entry.asynchronous
    )


def _source_matches(entry: ExpectedHook, row: dict[str, object]) -> bool:
    value = row.get("sourcePath")
    if not isinstance(value, str) or not Path(value).is_absolute():
        return False
    source = Path(value)
    if source.name != "hooks.json" or source.parent.name != "hooks":
        return False
    plugin = source.parent.parent
    try:
        identity = json.loads(
            (plugin / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
        )
    except (OSError, UnicodeError, ValueError):
        return False
    if not isinstance(identity, dict):
        return False
    return (
        identity.get("name") == entry.plugin_id.split("@", 1)[0]
        and identity.get("version") == entry.version
        and isinstance(identity.get("repository"), str)
        and identity["repository"].removesuffix(".git")
        == entry.repository.removesuffix(".git")
    )


def _command_matches(entry: ExpectedHook, row: dict[str, object]) -> bool:
    command = row.get("command")
    source = row.get("sourcePath")
    if not isinstance(command, str) or not isinstance(source, str):
        return False
    host_plugin = Path(source).parent.parent
    return _normalize_command(
        entry.command, entry.source_path.parent.parent
    ) == _normalize_command(command, host_plugin)


def _normalize_command(command: str, plugin: Path) -> str:
    return command.replace(str(plugin), "${PLUGIN_ROOT}")
