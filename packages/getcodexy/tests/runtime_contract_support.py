"""Shared fixtures for runtime release contract cases."""

import hashlib
import importlib
import json

TAG = "v1.3.0"
COMMIT = "a" * 40
ARCHIVE_DIGEST = "b" * 64
URL = f"https://github.com/eunsoogi/codexy/releases/download/{TAG}/codexy-runtime-package.tar.gz"
LEGACY_URL = f"https://github.com/eunsoogi/codexy/releases/download/{TAG}/codexy-marketplace-plugin.tar.gz"
BINARIES = {"lsp": b"lsp binary", "codegraph": b"codegraph binary"}


def contract_module():
    return importlib.import_module("codexy_runtime_tools.contract")


def encoded(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def candidate() -> dict[str, object]:
    platforms = {
        platform: {
            name: {
                "path": f"runtime/codexy-mcp-{name}-{platform}.{'exe' if platform == 'windows-x86_64' else 'bin'}",
                "sha256": hashlib.sha256(data).hexdigest(),
            }
            for name, data in BINARIES.items()
        }
        for platform in ("darwin-arm64", "linux-x86_64", "windows-x86_64")
    }
    return {
        "schema": "codexy-runtime-candidate/v1",
        "source": {
            "repository": "https://github.com/eunsoogi/codexy",
            "commit": COMMIT,
        },
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {
            "bootstrapApi": 1,
            "pluginRuntimeApi": 1,
            "transport": "stdio-newline-v1",
            "mcpProtocol": "2024-11-05",
        },
        "platforms": platforms,
    }


def release() -> dict[str, object]:
    embedded = candidate()
    return {
        "schema": "codexy-runtime-release/v1",
        "state": "candidate-proven",
        "source": embedded["source"],
        "artifact": {
            "tag": TAG,
            "url": URL,
            "sha256": ARCHIVE_DIGEST,
            "payloadManifestSha256": hashlib.sha256(encoded(embedded)).hexdigest(),
        },
        "compatibility": embedded["compatibility"],
        "platforms": embedded["platforms"],
    }


def legacy() -> dict[str, object]:
    value = release()
    value["state"] = "legacy-public"
    value["artifact"]["url"] = LEGACY_URL
    value["platforms"] = {
        platform: {
            server: {"sha256": binary["sha256"]} for server, binary in inventory.items()
        }
        for platform, inventory in candidate()["platforms"].items()
        if platform in ("darwin-arm64", "linux-x86_64")
    }
    return value
