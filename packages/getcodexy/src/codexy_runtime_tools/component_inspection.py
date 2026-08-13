"""Read-only component status and diagnostic reports."""

from __future__ import annotations

import os
import json
import subprocess
from pathlib import Path
from typing import Callable

from .component_manifest import ComponentManifest, load_component_manifest
from .component_resolver import ComponentResolutionError, admit_installed_inventory, canonical_components, classify_installed_inventory
from .component_transaction_state import read_inventory
from .github_pre_session import trusted_codex
from .plugin_resolution import named_marketplace, official_marketplace
from .pre_session import _find_codex, _json, _run
from .updater import _absolute, _validate_real_path


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]
STATUS_SCHEMA = "getcodexy.status.v1"
DOCTOR_SCHEMA = "getcodexy.doctor.v1"
SURFACE_PATHS = {
    "core": ("agents/catalog.toml", "hooks/hooks.json"),
    "github": ("agents/catalog.toml", "hooks/hooks.json"),
    "devtools": ("mcp/codexy-mcp-devtools",),
}


def status(codex_home: str | os.PathLike[str], *, codex: Path | None = None, runner: Runner | None = None) -> dict[str, object]:
    """Report live installed components without changing the Codex home."""
    report = _inspect(codex_home, codex, runner)
    return {
        "schema": STATUS_SCHEMA,
        "command": "status",
        "outcome": "completed",
        "inventory": report["inventory"],
        "inventory_consistency": report["consistency"],
        "selected_components": list(report["recorded"] or ()),
        "installed_components": list(report["actual"]),
        "source_of_truth": "installed-component-inventory",
        "errors": report["errors"],
    }


def doctor(codex_home: str | os.PathLike[str], *, codex: Path | None = None, runner: Runner | None = None) -> dict[str, object]:
    """Inspect the live component surface and return declarative repairs."""
    report = _inspect(codex_home, codex, runner)
    manifest = report["manifest"]
    actual, recorded = report["actual"], report["recorded"]
    health = _health(manifest, actual, recorded, report["records"], report["admission_error"])
    readiness = {"state": "ready", "missing_requirements": []}
    if report["host_error"]:
        readiness = {"state": "missing", "missing_requirements": ["codex-plugin-list"]}
    return {
        "schema": DOCTOR_SCHEMA,
        "command": "doctor",
        "outcome": "completed",
        "inventory": report["inventory"],
        "inventory_consistency": report["consistency"],
        "host_readiness": readiness,
        "component_health": health,
        "source_of_truth": "installed-component-inventory",
        "errors": report["errors"],
    }


def _inspect(codex_home: str | os.PathLike[str], codex: Path | None, runner: Runner | None) -> dict[str, object]:
    home = _absolute(codex_home)
    _validate_real_path(home, require_exists=False)
    manifest = load_component_manifest()
    recorded, inventory, inventory_error = _recorded(home)
    try:
        executable = trusted_codex(codex or _find_codex())
        invoke = runner or (lambda command: _run(command, home))
        installed = _json(invoke([str(executable), "plugin", "list", "--json"]), "plugin list")
        root = _marketplace_root(executable, invoke)
        actual, records, admission_error = _actual(manifest, installed, root)
        host_error = False
    except (OSError, RuntimeError, ValueError) as error:
        actual, records, admission_error, host_error = (), {}, _code(error), True
    errors = []
    error = admission_error or inventory_error
    if error:
        errors.append({"code": error})
    if admission_error or inventory_error or (recorded is not None and recorded != actual):
        consistency = "inconsistent"
        if not errors:
            errors.append({"code": "inconsistent-installed-state"})
    elif recorded is None:
        consistency = "not-recorded"
    else:
        consistency = "consistent"
    return {
        "manifest": manifest,
        "actual": actual,
        "recorded": recorded,
        "records": records,
        "admission_error": admission_error,
        "host_error": host_error,
        "inventory": inventory,
        "consistency": consistency,
        "errors": errors,
    }


def _recorded(home: Path) -> tuple[tuple[str, ...] | None, dict[str, object], str | None]:
    try:
        recorded = read_inventory(home)
    except (OSError, ValueError):
        return None, {"state": "invalid"}, "inconsistent-installed-state"
    if recorded is None:
        return None, {"state": "absent"}, None
    return recorded, {"state": "present", "components": list(recorded)}, None


def _marketplace_root(executable: Path, invoke: Runner) -> Path | None:
    payload = _json(invoke([str(executable), "plugin", "marketplace", "list", "--json"]), "plugin marketplace list")
    return official_marketplace(payload) if named_marketplace(payload) else None


def _actual(manifest: ComponentManifest, installed: object, root: Path | None) -> tuple[tuple[str, ...], dict[str, dict[str, object]], str | None]:
    actual: tuple[str, ...] = ()
    records: dict[str, dict[str, object]] = {}
    try:
        classified = classify_installed_inventory(manifest, installed)
        records = {}
        for record in classified.records:
            if record.component is not None:
                records.setdefault(record.component.id, record.entry)
        actual = canonical_components(manifest, set(records))
        admitted = admit_installed_inventory(manifest, installed, root)
        if root is None:
            return admitted, {}, None
        if actual != admitted:
            raise ComponentResolutionError("inconsistent-installed-state")
        return actual, records, None
    except ComponentResolutionError as error:
        return actual, records, error.code
    except (OSError, ValueError):
        return actual, records, "invalid-installed-inventory"


def _health(manifest: ComponentManifest, actual: tuple[str, ...], recorded: tuple[str, ...] | None, records: dict[str, dict[str, object]], admission_error: str | None) -> list[dict[str, str]]:
    expected = set(recorded or ()) | set(actual)
    result = []
    for component in manifest.component_ids:
        if component not in expected:
            continue
        if component not in actual:
            result.append(_entry(component, "missing"))
        elif _version_is_stale(manifest, component, records.get(component)):
            result.append(_entry(component, "stale"))
        elif admission_error and component in actual:
            result.append(_entry(component, "incompatible"))
        elif _stale(manifest, component, records.get(component)):
            result.append(_entry(component, "stale"))
        elif not set(manifest.component(component).dependencies).issubset(actual):
            result.append(_entry(component, "incompatible"))
        else:
            result.append({"component": component, "state": "healthy"})
    return result


def _entry(component: str, state: str) -> dict[str, str]:
    repair = "getcodexy bootstrap" if state in {"missing", "stale"} else "repair the Codexy registration, then rerun getcodexy doctor"
    return {"component": component, "state": state, "repair": repair}


def _version_is_stale(manifest: ComponentManifest, component: str, record: dict[str, object] | None) -> bool:
    return record is not None and isinstance(record.get("version"), str) and record["version"] < manifest.version


def _stale(manifest: ComponentManifest, component: str, record: dict[str, object] | None) -> bool:
    if record is None:
        return True
    source = record.get("source")
    root = source.get("path") if isinstance(source, dict) else None
    if not isinstance(root, str) or not Path(root).is_absolute():
        return True
    plugin = Path(root)
    required = manifest.component(component).asset.required_paths + SURFACE_PATHS[component]
    if any(not _regular(plugin / path) for path in required):
        return True
    if component == "devtools" and not os.access(plugin / "mcp/codexy-mcp-devtools", os.X_OK):
        return True
    return not _manifest_is_valid(plugin, manifest.component(component).plugin, manifest.version) or not _surface_json_is_valid(plugin, component) or _has_legacy_core_monolith(plugin, component)


def _regular(path: Path) -> bool:
    try:
        return path.is_file() and not path.is_symlink()
    except OSError:
        return False


def _surface_json_is_valid(plugin: Path, component: str) -> bool:
    paths = {"core": ("hooks/hooks.json",), "github": ("hooks/hooks.json",), "devtools": (".mcp.json",)}[component]
    return all(_json_object(plugin / path) for path in paths)


def _manifest_is_valid(plugin: Path, name: str, version: str) -> bool:
    value = _json_value(plugin / ".codex-plugin/plugin.json")
    return isinstance(value, dict) and value.get("name") == name and value.get("repository") == "https://github.com/eunsoogi/codexy" and value.get("version") == version


def _has_legacy_core_monolith(plugin: Path, component: str) -> bool:
    return component == "core" and any(os.path.lexists(plugin / path) for path in (".mcp.json", ".codex/lsp-client.json", "lsp", "mcp", "runtime-release.json"))


def _json_object(path: Path) -> bool:
    return isinstance(_json_value(path), dict)


def _json_value(path: Path) -> object | None:
    try:
        descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    except OSError:
        return None
    try:
        with os.fdopen(descriptor, "r", encoding="utf-8", closefd=False) as source:
            return json.load(source)
    except (OSError, ValueError, json.JSONDecodeError):
        return None
    finally:
        os.close(descriptor)


def _code(error: BaseException) -> str:
    return error.code if isinstance(error, ComponentResolutionError) else "invalid-installed-inventory"
