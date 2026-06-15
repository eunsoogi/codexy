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


def load_lsp_catalog(plugin_root: Path) -> dict[str, dict[str, Any]] | None:
    catalog_path = plugin_root / "lsp" / "server-catalog.toml"
    if not catalog_path.exists():
        return None
    catalog = load_toml(catalog_path)
    servers = catalog.get("servers")
    if not isinstance(servers, list):
        raise ValidationError(f"{rel(catalog_path)} must contain [[servers]] entries")
    if not servers:
        raise ValidationError(f"{rel(catalog_path)} must contain at least one [[servers]] entry")

    known: dict[str, dict[str, Any]] = {}
    for index, server in enumerate(servers, start=1):
        if not isinstance(server, dict):
            raise ValidationError(f"{rel(catalog_path)} servers[{index}] must be a table")
        server_id = require_string(server.get("id"), f"servers[{index}].id", catalog_path)
        if server_id in known:
            raise ValidationError(f"{rel(catalog_path)} duplicate server id: {server_id}")
        extensions = server.get("extensions")
        if not isinstance(extensions, list) or not all(isinstance(item, str) for item in extensions):
            raise ValidationError(f"{rel(catalog_path)} {server_id}.extensions must be a list of strings")
        command = server.get("command")
        if not isinstance(command, list) or not command or not all(isinstance(item, str) and item for item in command):
            raise ValidationError(f"{rel(catalog_path)} {server_id}.command must be a non-empty argv array")
        if "args" in server:
            raise ValidationError(f"{rel(catalog_path)} {server_id}.args is not allowed; include argv in command")
        known[server_id] = {
            "extensions": set(extensions),
            "command": list(command),
        }
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
        command = entry.get("command")
        if command is not None and (
            not isinstance(command, list) or not command or not all(isinstance(item, str) and item for item in command)
        ):
            raise ValidationError(f"{rel(lsp_path)} {server_id}.command must be a non-empty argv array")
        if "args" in entry:
            raise ValidationError(f"{rel(lsp_path)} {server_id}.args is not allowed; include argv in command")
    return entries


def covered_extensions(entries: dict[str, dict[str, Any]]) -> set[str]:
    covered: set[str] = set()
    for entry in entries.values():
        covered.update(entry["extensions"])
    return covered


def catalog_covered_extensions(entries: dict[str, dict[str, Any]], catalog: dict[str, dict[str, Any]]) -> set[str]:
    covered: set[str] = set()
    for server_id, entry in entries.items():
        catalog_entry = catalog.get(server_id)
        if catalog_entry is None:
            continue
        catalog_extensions = catalog_entry["extensions"]
        covered.update(extension for extension in entry["extensions"] if extension in catalog_extensions)
    return covered


def check_lsp(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    try:
        entries = lsp_entries(plugin_root)
        catalog = load_lsp_catalog(plugin_root)
        coverage_for_missing = covered_extensions(entries)
        if catalog is None:
            errors.append("LSP coverage requires lsp/server-catalog.toml")
        else:
            for server_id, entry in entries.items():
                if server_id not in catalog:
                    errors.append(f"LSP server {server_id!r} is not present in lsp/server-catalog.toml")
                    continue
                catalog_entry = catalog[server_id]
                undeclared = sorted(set(entry["extensions"]) - catalog_entry["extensions"])
                if undeclared:
                    errors.append(
                        f"LSP server {server_id!r} configures extensions not declared by catalog: "
                        f"{', '.join(undeclared)}"
                    )
                if entry.get("command") != catalog_entry["command"]:
                    errors.append(
                        f"LSP server {server_id!r} must define command argv {catalog_entry['command']!r} "
                        "from lsp/server-catalog.toml"
                    )
            coverage_for_missing = catalog_covered_extensions(entries, catalog)
        missing = sorted(REQUIRED_LSP_EXTENSIONS - coverage_for_missing)
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
            elif command is not None and "context7" in command.lower():
                errors.append(f"{rel(path)} disallowed context7 MCP command present for {name}")
    except ValidationError as exc:
        errors.append(str(exc))
    return errors


def check_specialist_agent_files(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    agents_root = plugin_root / "agents"
    legacy_roles_dir = agents_root / "roles"
    legacy_roles_path = agents_root / "roles.toml"
    catalog_path = agents_root / "catalog.toml"
    if legacy_roles_dir.exists():
        errors.append(
            f"{rel(legacy_roles_dir)} must not contain specialist agent definitions; "
            "store each specialist agent in agents/<name>.toml"
        )
    if legacy_roles_path.exists():
        errors.append(
            f"{rel(legacy_roles_path)} must not contain collapsed multi-role metadata; "
            "store each specialist agent in agents/<name>.toml"
        )
    if not catalog_path.exists():
        return errors + [f"{rel(catalog_path)} is required for specialist agent discovery metadata"]
    try:
        catalog = load_toml(catalog_path)
    except ValidationError as exc:
        return [str(exc)]

    prefix = catalog.get("default_branch_prefix")
    if prefix in DISALLOWED_BRANCH_PREFIXES:
        errors.append(f"{rel(catalog_path)} default_branch_prefix must not be {prefix!r}")
    agent_files_glob = catalog.get("agent_files_glob")
    if agent_files_glob != "*.toml":
        errors.append(f"{rel(catalog_path)} agent_files_glob must be '*.toml'")
    agent_files = sorted(
        path
        for path in agents_root.glob("*.toml")
        if path.name != "catalog.toml"
    )
    if not agent_files:
        errors.append(f"{rel(agents_root)} must contain one TOML file per specialist agent")
        return errors
    seen: set[str] = set()
    for path in agent_files:
        try:
            agent = load_toml(path)
        except ValidationError as exc:
            errors.append(str(exc))
            continue
        if "roles" in agent:
            errors.append(f"{rel(path)} must define exactly one specialist agent and must not contain [[roles]]")
        name = agent.get("name")
        if not isinstance(name, str) or not name:
            errors.append(f"{rel(path)} name must be a non-empty string")
            continue
        if path.stem != name:
            errors.append(f"{rel(path)} filename must match agent name {name!r}")
        if name in seen:
            errors.append(f"{rel(path)} duplicate agent name: {name}")
        seen.add(name)
        if name == "orchestrator":
            errors.append(f"{rel(path)} assignable child orchestrator agent is not allowed")
        for field in ("display_name", "model", "effort", "when_to_use"):
            if not isinstance(agent.get(field), str) or not agent.get(field):
                errors.append(f"{rel(path)} {field} must be a non-empty string")
        for field in ("inputs", "outputs", "constraints"):
            value = agent.get(field)
            if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
                errors.append(f"{rel(path)} {field} must be a list of non-empty strings")
    required = {
        "planner",
        "explorer",
        "architect",
        "implementer",
        "debugger",
        "qa",
        "reviewer",
        "integrator",
        "release",
        "security",
        "documenter",
    }
    missing = sorted(required - seen)
    if missing:
        errors.append(f"{rel(agents_root)} missing specialist agents: {', '.join(missing)}")
    return errors


def check_project_agents(plugin_root: Path) -> list[str]:
    errors: list[str] = []
    agents_dir = plugin_root / ".codex" / "agents"
    if not agents_dir.exists():
        return errors
    errors.append(
        f"{rel(agents_dir)} is not loaded from an installed plugin; "
        "keep plugin-packaged specialist agent definitions in agents/<name>.toml"
    )
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
    child_indents: dict[int, int] = {}
    previous_indent = -1
    previous_was_mapping = True
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
        if indent > previous_indent and not previous_was_mapping:
            raise ValidationError(f"{rel(path)} line {line_number} cannot be nested under a scalar value")
        key, raw_value = stripped.split(":", 1)
        key = key.strip()
        if not key:
            raise ValidationError(f"{rel(path)} line {line_number} has an empty key")
        while stack and indent <= stack[-1][0]:
            stack.pop()
        if not stack:
            raise ValidationError(f"{rel(path)} line {line_number} has invalid indentation")
        parent = stack[-1][1]
        expected_indent = child_indents.get(id(parent))
        if expected_indent is None:
            child_indents[id(parent)] = indent
        elif expected_indent != indent:
            raise ValidationError(f"{rel(path)} line {line_number} has inconsistent sibling indentation")
        value_text = raw_value.strip()
        if not value_text:
            child: dict[str, Any] = {}
            parent[key] = child
            stack.append((indent, child))
            previous_indent = indent
            previous_was_mapping = True
            continue
        parent[key] = parse_prompt_yaml_scalar(value_text)
        previous_indent = indent
        previous_was_mapping = False
    return root


def parse_prompt_yaml_scalar(value: str) -> Any:
    if value == "true":
        return True
    if value == "false":
        return False
    starts_quote = value.startswith(('"', "'"))
    ends_quote = value.endswith(('"', "'"))
    if starts_quote != ends_quote:
        raise ValidationError("quoted scalar is unterminated")
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
    errors.extend(check_specialist_agent_files(plugin_root))
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
