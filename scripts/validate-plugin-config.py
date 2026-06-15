#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PLUGIN_ROOT = REPO_ROOT / "plugins" / "codexy"
REQUIRED_LSP_EXTENSIONS = {".py", ".pyi", ".yaml", ".yml", ".json", ".toml", ".md"}
DISALLOWED_MCP_NAMES = {"context7"}
DISALLOWED_BRANCH_PREFIXES = {"eunsoogi/"}


class ValidationError(RuntimeError):
    pass


def rel(path: Path) -> str:
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def load_json(path: Path) -> Any:
    try:
        with path.open(encoding="utf-8") as handle:
            return json.load(handle)
    except FileNotFoundError as exc:
        raise ValidationError(f"missing required file: {rel(path)}") from exc
    except json.JSONDecodeError as exc:
        raise ValidationError(f"invalid JSON in {rel(path)}: {exc}") from exc


def load_toml(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as handle:
            data = tomllib.load(handle)
    except FileNotFoundError as exc:
        raise ValidationError(f"missing TOML file: {rel(path)}") from exc
    except tomllib.TOMLDecodeError as exc:
        raise ValidationError(f"invalid TOML in {rel(path)}: {exc}") from exc
    if not isinstance(data, dict):
        raise ValidationError(f"{rel(path)} must parse to a TOML table")
    return data


def require_string(value: Any, field: str, path: Path) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValidationError(f"{rel(path)} {field} must be a non-empty string")
    return value


def manifest_path(plugin_root: Path) -> Path:
    return plugin_root / ".codex-plugin" / "plugin.json"


def load_manifest(plugin_root: Path) -> dict[str, Any]:
    manifest = load_json(manifest_path(plugin_root))
    if not isinstance(manifest, dict):
        raise ValidationError(f"{rel(manifest_path(plugin_root))} must contain a JSON object")
    require_string(manifest.get("name"), "name", manifest_path(plugin_root))
    require_string(manifest.get("version"), "version", manifest_path(plugin_root))
    interface = manifest.get("interface")
    if not isinstance(interface, dict):
        raise ValidationError(f"{rel(manifest_path(plugin_root))} interface must be an object")
    require_string(interface.get("displayName"), "interface.displayName", manifest_path(plugin_root))
    require_string(interface.get("shortDescription"), "interface.shortDescription", manifest_path(plugin_root))
    default_prompt = interface.get("defaultPrompt")
    if not isinstance(default_prompt, list) or not default_prompt:
        raise ValidationError(f"{rel(manifest_path(plugin_root))} interface.defaultPrompt must be a non-empty array")
    if not all(isinstance(item, str) and item.strip() for item in default_prompt):
        raise ValidationError(
            f"{rel(manifest_path(plugin_root))} interface.defaultPrompt must contain only non-empty strings"
        )
    return manifest


def mcp_config_path(plugin_root: Path, manifest: dict[str, Any]) -> Path:
    configured = manifest.get("mcpServers")
    if not isinstance(configured, str) or not configured:
        raise ValidationError(f"{rel(manifest_path(plugin_root))} mcpServers must be a path string")
    configured_path = Path(configured)
    if configured_path.is_absolute():
        raise ValidationError(f"{rel(manifest_path(plugin_root))} mcpServers must be plugin-relative")
    resolved = (plugin_root / configured_path).resolve()
    plugin_root_resolved = plugin_root.resolve()
    if not resolved.is_relative_to(plugin_root_resolved):
        raise ValidationError(f"{rel(manifest_path(plugin_root))} mcpServers must stay inside the plugin root")
    return resolved


def load_lsp_catalog(plugin_root: Path) -> dict[str, set[str]]:
    catalog_path = plugin_root / "lsp" / "server-catalog.toml"
    if not catalog_path.exists():
        return {}
    catalog = load_toml(catalog_path)
    servers = catalog.get("servers")
    if not isinstance(servers, list):
        raise ValidationError(f"{rel(catalog_path)} must contain [[servers]] entries")
    if not servers:
        raise ValidationError(f"{rel(catalog_path)} must contain at least one [[servers]] entry")

    known: dict[str, set[str]] = {}
    for index, server in enumerate(servers, start=1):
        if not isinstance(server, dict):
            raise ValidationError(f"{rel(catalog_path)} servers[{index}] must be a table")
        server_id = require_string(server.get("id"), f"servers[{index}].id", catalog_path)
        if server_id in known:
            raise ValidationError(f"{rel(catalog_path)} duplicate server id: {server_id}")
        extensions = server.get("extensions")
        if not isinstance(extensions, list) or not all(isinstance(item, str) for item in extensions):
            raise ValidationError(f"{rel(catalog_path)} {server_id}.extensions must be a list of strings")
        known[server_id] = set(extensions)
    return known


def lsp_entries(plugin_root: Path) -> dict[str, dict[str, Any]]:
    lsp_path = plugin_root / ".codex" / "lsp-client.json"
    data = load_json(lsp_path)
    if not isinstance(data, dict):
        raise ValidationError(f"{rel(lsp_path)} must contain a JSON object")
    entries = data.get("lsp")
    if not isinstance(entries, dict):
        raise ValidationError(f"{rel(lsp_path)} must contain an object at key 'lsp'")
    for server_id, entry in entries.items():
        if not isinstance(server_id, str) or not server_id:
            raise ValidationError(f"{rel(lsp_path)} LSP server ids must be non-empty strings")
        if not isinstance(entry, dict):
            raise ValidationError(f"{rel(lsp_path)} {server_id} must be an object")
        extensions = entry.get("extensions")
        if not isinstance(extensions, list) or not all(isinstance(item, str) for item in extensions):
            raise ValidationError(f"{rel(lsp_path)} {server_id}.extensions must be a list of strings")
        if not extensions:
            raise ValidationError(f"{rel(lsp_path)} {server_id}.extensions must not be empty")
        priority = entry.get("priority")
        if type(priority) is not int:
            raise ValidationError(f"{rel(lsp_path)} {server_id}.priority must be an integer")
    return entries


def covered_extensions(entries: dict[str, dict[str, Any]]) -> set[str]:
    covered: set[str] = set()
    for entry in entries.values():
        covered.update(entry["extensions"])
    return covered


def catalog_covered_extensions(entries: dict[str, dict[str, Any]], catalog: dict[str, set[str]]) -> set[str]:
    if not catalog:
        return covered_extensions(entries)
    covered: set[str] = set()
    for server_id, entry in entries.items():
        covered.update(extension for extension in entry["extensions"] if extension in catalog.get(server_id, set()))
    return covered


def check_lsp(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    try:
        entries = lsp_entries(plugin_root)
        catalog = load_lsp_catalog(plugin_root)
        if catalog:
            for server_id, entry in entries.items():
                if server_id not in catalog:
                    errors.append(f"LSP server {server_id!r} is not present in lsp/server-catalog.toml")
                    continue
                undeclared = sorted(set(entry["extensions"]) - catalog[server_id])
                if undeclared:
                    errors.append(
                        f"LSP server {server_id!r} configures extensions not declared by catalog: "
                        f"{', '.join(undeclared)}"
                    )
        missing = sorted(REQUIRED_LSP_EXTENSIONS - catalog_covered_extensions(entries, catalog))
        if missing:
            errors.append(f"LSP coverage missing required extensions: {', '.join(missing)}")
    except ValidationError as exc:
        errors.append(str(exc))
    return errors


def check_mcp(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    try:
        manifest = load_manifest(plugin_root)
        path = mcp_config_path(plugin_root, manifest)
        data = load_json(path)
        if not isinstance(data, dict):
            raise ValidationError(f"{rel(path)} must contain a JSON object")
        for name, entry in data.items():
            if not isinstance(name, str) or not name:
                errors.append(f"{rel(path)} MCP names must be non-empty strings")
                continue
            if name in DISALLOWED_MCP_NAMES:
                errors.append(f"{rel(path)} disallowed MCP server present: {name}")
            if not isinstance(entry, dict):
                errors.append(f"{rel(path)} {name} must be an object")
                continue
            url = entry.get("url")
            command = entry.get("command")
            if url is None and command is None:
                errors.append(f"{rel(path)} {name} must define either url or command")
            if url is not None:
                if not isinstance(url, str) or not url.startswith(("https://", "http://")):
                    errors.append(f"{rel(path)} {name}.url must be an HTTP(S) string")
                if "context7" in str(url).lower():
                    errors.append(f"{rel(path)} disallowed context7 MCP URL present for {name}")
            if command is not None and not isinstance(command, str):
                errors.append(f"{rel(path)} {name}.command must be a string")
    except ValidationError as exc:
        errors.append(str(exc))
    return errors


def check_legacy_roles(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    roles_path = plugin_root / "agents" / "roles.toml"
    if not roles_path.exists():
        return errors
    try:
        data = load_toml(roles_path)
    except ValidationError as exc:
        return [str(exc)]

    prefix = data.get("default_branch_prefix")
    if prefix in DISALLOWED_BRANCH_PREFIXES:
        errors.append(f"{rel(roles_path)} default_branch_prefix must not be {prefix!r}")
    roles = data.get("roles")
    if roles is not None and not isinstance(roles, list):
        errors.append(f"{rel(roles_path)} roles must be an array of tables")
        return errors
    for index, role in enumerate(roles or [], start=1):
        if not isinstance(role, dict):
            errors.append(f"{rel(roles_path)} roles[{index}] must be a table")
            continue
        name = role.get("name")
        if not isinstance(name, str) or not name:
            errors.append(f"{rel(roles_path)} roles[{index}].name must be a non-empty string")
        if name == "orchestrator":
            errors.append(f"{rel(roles_path)} assignable child orchestrator role is not allowed")
    return errors


def check_project_agents(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    agents_dir = plugin_root / ".codex" / "agents"
    if not agents_dir.exists():
        return errors
    seen: set[str] = set()
    for path in sorted(agents_dir.glob("*.toml")):
        try:
            data = load_toml(path)
            name = require_string(data.get("name"), "name", path)
            require_string(data.get("description"), "description", path)
            require_string(data.get("developer_instructions"), "developer_instructions", path)
        except ValidationError as exc:
            errors.append(str(exc))
            continue
        if name in seen:
            errors.append(f"{rel(path)} duplicate custom agent name: {name}")
        seen.add(name)
        if name == "orchestrator":
            errors.append(f"{rel(path)} must not define a child orchestrator agent")
        nicknames = data.get("nickname_candidates")
        if nicknames is not None:
            if not isinstance(nicknames, list) or not all(isinstance(item, str) and item for item in nicknames):
                errors.append(f"{rel(path)} nickname_candidates must be a list of non-empty strings")
            elif len(set(nicknames)) != len(nicknames):
                errors.append(f"{rel(path)} nickname_candidates must be unique")
    return errors


def check_agent_yaml_file(path: Path) -> list[str]:
    errors: list[str] = []
    text = path.read_text(encoding="utf-8")
    if "\t" in text:
        errors.append(f"{rel(path)} must not contain tab indentation")
    try:
        data = parse_prompt_yaml(text, path)
    except ValidationError as exc:
        return [str(exc)]
    interface = data.get("interface")
    if not isinstance(interface, dict):
        errors.append(f"{rel(path)} interface must be a mapping")
        interface = {}
    policy = data.get("policy")
    if not isinstance(policy, dict):
        errors.append(f"{rel(path)} policy must be a mapping")
        policy = {}
    for field in ("display_name", "short_description", "default_prompt"):
        value = interface.get(field)
        if not isinstance(value, str) or not value.strip():
            errors.append(f"{rel(path)} interface.{field} must be a non-empty string")
    if policy.get("allow_implicit_invocation") is not True:
        errors.append(f"{rel(path)} policy.allow_implicit_invocation must be true")
    return errors


def parse_prompt_yaml(text: str, path: Path) -> dict[str, Any]:
    root: dict[str, Any] = {}
    stack: list[tuple[int, dict[str, Any]]] = [(-1, root)]
    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        if not raw_line.strip() or raw_line.lstrip().startswith("#"):
            continue
        if raw_line.startswith(" "):
            indent = len(raw_line) - len(raw_line.lstrip(" "))
        else:
            indent = 0
        stripped = raw_line.strip()
        if ":" not in stripped:
            raise ValidationError(f"{rel(path)} line {line_number} must be a YAML key/value pair")
        key, raw_value = stripped.split(":", 1)
        key = key.strip()
        if not key:
            raise ValidationError(f"{rel(path)} line {line_number} has an empty key")
        while stack and indent <= stack[-1][0]:
            stack.pop()
        if not stack:
            raise ValidationError(f"{rel(path)} line {line_number} has invalid indentation")
        parent = stack[-1][1]
        value_text = raw_value.strip()
        if not value_text:
            child: dict[str, Any] = {}
            parent[key] = child
            stack.append((indent, child))
            continue
        parent[key] = parse_prompt_yaml_scalar(value_text)
    return root


def parse_prompt_yaml_scalar(value: str) -> Any:
    if value == "true":
        return True
    if value == "false":
        return False
    if (value.startswith('"') and value.endswith('"')) or (value.startswith("'") and value.endswith("'")):
        return value[1:-1]
    return value


def check_agent_yaml(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    for skill_file in sorted((plugin_root / "skills").glob("*/SKILL.md")):
        prompt_file = skill_file.parent / "agents" / "openai.yaml"
        if not prompt_file.exists():
            errors.append(f"{rel(skill_file.parent)} skill bundle is missing agents/openai.yaml")
    yaml_files = sorted(plugin_root.glob("**/agents/openai.yaml"))
    if not yaml_files:
        return [f"{rel(plugin_root)} has no agents/openai.yaml files"]
    for path in yaml_files:
        try:
            errors.extend(check_agent_yaml_file(path))
        except OSError as exc:
            errors.append(f"cannot read {rel(path)}: {exc}")
    return errors


def check_roles(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    errors.extend(check_legacy_roles(plugin_root))
    errors.extend(check_project_agents(plugin_root))
    errors.extend(check_agent_yaml(plugin_root))
    return errors


def check_manifest(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    try:
        manifest = load_manifest(plugin_root)
        mcp_path = mcp_config_path(plugin_root, manifest)
        if not mcp_path.exists():
            errors.append(f"manifest mcpServers target does not exist: {rel(mcp_path)}")
    except ValidationError as exc:
        errors.append(str(exc))
    return errors


def run_checks(plugin_root: Path, mode: str) -> list[str]:
    if mode == "lsp":
        return check_lsp(plugin_root)
    if mode == "mcp":
        return check_mcp(plugin_root)
    if mode == "roles":
        return check_roles(plugin_root)
    errors: list[str] = []
    errors.extend(check_manifest(plugin_root))
    errors.extend(check_lsp(plugin_root))
    errors.extend(check_mcp(plugin_root))
    errors.extend(check_roles(plugin_root))
    return errors


def print_covered_extensions(plugin_root: Path) -> None:
    entries = lsp_entries(plugin_root)
    for extension in sorted(covered_extensions(entries)):
        print(extension)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate Codexy plugin configuration surfaces.")
    parser.add_argument(
        "--plugin-root",
        type=Path,
        default=DEFAULT_PLUGIN_ROOT,
        help="plugin root to validate (default: plugins/codexy)",
    )
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--check", action="store_true", help="validate all plugin config surfaces")
    modes.add_argument("--check-lsp", action="store_true", help="validate LSP config only")
    modes.add_argument("--check-mcp", action="store_true", help="validate MCP config only")
    modes.add_argument("--check-roles", action="store_true", help="validate role and agent prompt config only")
    modes.add_argument("--print-covered-extensions", action="store_true", help="print sorted LSP covered extensions")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    plugin_root = args.plugin_root.resolve()
    if args.print_covered_extensions:
        try:
            print_covered_extensions(plugin_root)
        except ValidationError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 1
        return 0

    mode = "all"
    if args.check_lsp:
        mode = "lsp"
    elif args.check_mcp:
        mode = "mcp"
    elif args.check_roles:
        mode = "roles"

    errors = run_checks(plugin_root, mode)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"plugin config validation ok: {rel(plugin_root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
