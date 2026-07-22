"""Strict JSON and normalized runtime inventory primitives."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from typing import Any


_DIGEST = re.compile(r"[0-9a-f]{64}\Z")
PLATFORMS = {"darwin-arm64", "linux-x86_64"}
SERVERS = {"lsp", "codegraph"}


@dataclass(frozen=True)
class Compatibility:
    bootstrap_api: int
    plugin_runtime_api: int
    transport: str
    mcp_protocol: str


def document(text: str) -> Any:
    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in items:
            if key in value:
                raise ValueError(f"runtime release has duplicate JSON key: {key}")
            value[key] = item
        return value
    return json.loads(text, object_pairs_hook=pairs)


def object(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"runtime release {name} must be an object")
    return value


def string(value: Any, name: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"runtime release {name} must be a non-empty string")
    return value


def digest(value: Any, name: str) -> str:
    value = string(value, name)
    if not _DIGEST.fullmatch(value):
        raise ValueError(f"runtime release {name} must be a lowercase SHA-256")
    return value


def compatibility(value: Any) -> Compatibility:
    value = object(value, "compatibility")
    if set(value) != {"bootstrapApi", "pluginRuntimeApi", "transport", "mcpProtocol"}:
        raise ValueError("runtime release compatibility has unknown or missing fields")
    if value.get("bootstrapApi") != 1 or value.get("pluginRuntimeApi") != 1:
        raise ValueError("runtime release compatibility APIs must be 1")
    if value.get("transport") != "stdio-newline-v1" or value.get("mcpProtocol") != "2024-11-05":
        raise ValueError("runtime release compatibility transport or MCP protocol is unsupported")
    return Compatibility(1, 1, "stdio-newline-v1", "2024-11-05")


def platforms(value: Any, *, require_path: bool) -> dict[str, dict[str, dict[str, str]]]:
    value = object(value, "platforms")
    if set(value) != PLATFORMS:
        raise ValueError("runtime release has unknown or missing platform")
    result: dict[str, dict[str, dict[str, str]]] = {}
    for platform, inventory in value.items():
        inventory = object(inventory, f"platforms.{platform}")
        if set(inventory) != SERVERS:
            raise ValueError("runtime release has unknown or missing server")
        binaries: dict[str, dict[str, str]] = {}
        for server, item in inventory.items():
            item = object(item, "binary")
            fields = {"path", "sha256"} if require_path else {"sha256"}
            if set(item) != fields:
                raise ValueError("runtime release binary has unknown or missing fields")
            binary = {"sha256": digest(item.get("sha256"), "binary.sha256")}
            if require_path:
                path = string(item.get("path"), "binary.path")
                expected = f"runtime/codexy-mcp-{server}-{platform}.bin"
                if path != expected or path.casefold() != path:
                    raise ValueError("runtime release binary path is not canonical")
                binary["path"] = path
            binaries[server] = binary
        result[platform] = binaries
    return result
