"""Read-only validation of managed registrations through admitted no-follow trees."""

from __future__ import annotations

import tomllib
from dataclasses import dataclass

from .component_source_admission import DiagnosticFailure, DiagnosticTree


CATALOGS = {
    "core": {
        "version": "0.1.0",
        "default_branch_prefix": "codexy/",
        "catalog_kind": "plugin-packaged-specialist-agent-files",
        "native_custom_agent_registration": "codex-home-standalone-agent-projection",
        "native_custom_agent_projection": "managed-codexy-subdirectory",
        "agent_files": [
            "codexy-architect.toml",
            "codexy-cartographer.toml",
            "codexy-auditor.toml",
            "codexy-shipwright.toml",
            "codexy-inspector.toml",
            "codexy-sentinel.toml",
            "codexy-warden.toml",
        ],
    },
    "github": {
        "version": "0.1.0",
        "catalog_kind": "plugin-packaged-specialist-agent-files",
        "agent_files": ["codexy-weaver.toml"],
    },
}
HOOKS = {
    "core": {
        "hooks": {
            "PermissionRequest": [{
                "matcher": "^codex_app__send_message_to_thread$",
                "hooks": [{
                    "type": "command",
                    "command": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.sh\" PermissionRequest",
                    "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.cmd\" PermissionRequest",
                    "timeout": 5,
                }],
            }],
            "PreToolUse": [{
                "matcher": "^codex_app__send_message_to_thread$",
                "hooks": [{
                    "type": "command",
                    "command": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.sh\" PreToolUse",
                    "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.cmd\" PreToolUse",
                    "timeout": 5,
                }],
            }],
        },
    },
    "github": {
        "hooks": {
            "UserPromptSubmit": [{"hooks": [{
                "type": "command",
                "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.sh\"",
                "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.cmd\"",
                "timeout": 5,
            }]}],
            "PreToolUse": [
                {
                    "matcher": "^mcp__codex_apps__github_(create|update)_issue$",
                    "hooks": [{
                        "type": "command",
                        "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh\" --rule issue",
                        "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission-issue.cmd\"",
                        "timeout": 5,
                    }],
                },
                {
                    "matcher": "^mcp__codex_apps__github_create_pull_request$",
                    "hooks": [{
                        "type": "command",
                        "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh\" --rule pr",
                        "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission-pr.cmd\"",
                        "timeout": 5,
                    }],
                },
            ],
        },
    },
}
MCP = {
    "lsp": {"command": "./mcp/codexy-mcp-devtools", "args": ["lsp", "--stdio"], "cwd": "."},
    "codegraph": {"command": "./mcp/codexy-mcp-devtools", "args": ["codegraph", "--stdio"], "cwd": "."},
}
SURFACE_PATHS = {
    "core": ("agents/catalog.toml", "hooks/hooks.json", "hooks/codexy-thread-delivery.sh", "hooks/codexy-thread-delivery.cmd"),
    "github": (
        "agents/catalog.toml",
        "hooks/hooks.json",
        "hooks/codexy-github-workflow-context.sh",
        "hooks/codexy-github-workflow-context.cmd",
        "hooks/codexy-github-admission.sh",
        "hooks/codexy-github-admission-issue.cmd",
        "hooks/codexy-github-admission-pr.cmd",
    ),
    "devtools": ("mcp/codexy-mcp-devtools", ".mcp.json"),
}


@dataclass(frozen=True)
class SurfaceDiagnosis:
    canonical: bool
    failure: DiagnosticFailure | None = None


def valid_surface(tree: DiagnosticTree, component: str) -> bool:
    diagnosis = diagnose_surface(tree, component)
    return diagnosis.canonical and diagnosis.failure is None


def diagnose_surface(tree: DiagnosticTree, component: str) -> SurfaceDiagnosis:
    if failure := _read_failure(tree, SURFACE_PATHS[component]):
        return SurfaceDiagnosis(False, failure)
    if component == "devtools":
        launcher = tree.read("mcp/codexy-mcp-devtools")
        value, failure = _json_value(tree, ".mcp.json")
        return SurfaceDiagnosis(launcher.executable and value == MCP, failure)
    catalog, failure = _toml_value(tree, "agents/catalog.toml")
    if failure:
        return SurfaceDiagnosis(False, failure)
    if failure := _read_failure(tree, tuple(f"agents/{name}" for name in CATALOGS[component]["agent_files"])):
        return SurfaceDiagnosis(False, failure)
    hooks, failure = _json_value(tree, "hooks/hooks.json")
    return SurfaceDiagnosis(catalog == CATALOGS[component] and hooks == HOOKS[component], failure)


def _read_failure(tree: DiagnosticTree, paths: tuple[str, ...]) -> DiagnosticFailure | None:
    return next((read.failure for path in paths if (read := tree.read(path)).failure), None)


def _json_value(tree: DiagnosticTree, relative: str) -> tuple[object | None, DiagnosticFailure | None]:
    read = tree.read(relative)
    if read.failure:
        return None, read.failure
    try:
        return loads(read.contents, object_pairs_hook=_unique_object), None  # type: ignore[arg-type]
    except (UnicodeDecodeError, ValueError):
        return None, DiagnosticFailure.MALFORMED


def _toml_value(tree: DiagnosticTree, relative: str) -> tuple[object | None, DiagnosticFailure | None]:
    read = tree.read(relative)
    if read.failure:
        return None, read.failure
    try:
        return tomllib.loads(read.contents.decode()), None  # type: ignore[union-attr]
    except (UnicodeDecodeError, tomllib.TOMLDecodeError):
        return None, DiagnosticFailure.MALFORMED


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("registration has duplicate keys")
        result[key] = value
    return result
from .component_json import loads
