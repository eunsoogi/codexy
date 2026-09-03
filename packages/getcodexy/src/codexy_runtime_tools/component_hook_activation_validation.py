"""Classify effective host hook rows against installed plugin registrations."""

from __future__ import annotations

import json
from pathlib import Path
from typing import cast

from .component_hook_activation_expected import ExpectedHook
from .component_hook_activation_host import normalize_hook_rows


def classify_activation(
    expected: dict[str, tuple[ExpectedHook, ...]],
    rows: object,
    *,
    codex_home: Path | None = None,
) -> dict[str, str]:
    observed = normalize_hook_rows(rows)
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
            if not _identity_matches(entry, row, codex_home):
                failures[component] = "required-hook-trust-stale"
                break
    return failures


def _identity_matches(
    entry: ExpectedHook, row: dict[str, object], codex_home: Path | None
) -> bool:
    return (
        row.get("key") == entry.key
        and row.get("pluginId") == entry.plugin_id
        and _source_matches(entry, row, codex_home)
        and row.get("eventName") == entry.event_name
        and row.get("handlerType") == "command"
        and row.get("matcher") == entry.matcher
        and _command_matches(entry, row)
        and row.get("timeoutSec") == entry.timeout
        and row.get("async") is entry.asynchronous
    )


def _source_matches(
    entry: ExpectedHook, row: dict[str, object], codex_home: Path | None
) -> bool:
    value = row.get("sourcePath")
    if not isinstance(value, str) or not Path(value).is_absolute():
        return False
    source = Path(value)
    if source.name != "hooks.json" or source.parent.name != "hooks":
        return False
    if source.is_symlink() or not _allowed_source(source, entry, codex_home):
        return False
    plugin = source.parent.parent
    try:
        if source.read_bytes() != entry.source_path.read_bytes():
            return False
        identity_value = cast(
            object,
            json.loads(
                (plugin / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
            ),
        )
        if not isinstance(identity_value, dict):
            return False
        identity = cast(
            dict[str, object],
            identity_value,
        )
    except (OSError, UnicodeError, ValueError):
        return False
    repository = identity.get("repository")
    if not isinstance(repository, str):
        return False
    return (
        identity.get("name") == entry.plugin_id.split("@", 1)[0]
        and identity.get("version") == entry.version
        and repository.removesuffix(".git") == entry.repository.removesuffix(".git")
    )


def _allowed_source(source: Path, entry: ExpectedHook, codex_home: Path | None) -> bool:
    if source == entry.source_path:
        return True
    if codex_home is None:
        return False
    plugin, _, marketplace = entry.plugin_id.partition("@")
    return bool(marketplace) and source == (
        codex_home
        / "plugins"
        / "cache"
        / marketplace
        / plugin
        / entry.version
        / "hooks"
        / "hooks.json"
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
