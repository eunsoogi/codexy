"""Explicit protocol, transport, platform, and SDK support contract."""

from __future__ import annotations

import os
from dataclasses import dataclass


DEFAULT_PROTOCOL_VERSION = "2024-11-05"
DEFAULT_TRANSPORT = "stdio-newline-v1"
MAX_REQUEST_BYTES = 64 * 1024
MAX_OUTPUT_BYTES = 1024 * 1024
SUPPORTED_PROTOCOL_VERSIONS = (DEFAULT_PROTOCOL_VERSION,)
SUPPORTED_TRANSPORTS = (DEFAULT_TRANSPORT,)
SUPPORTED_PLATFORMS = ("posix",)
PLATFORM_LIMITATIONS = (
    "Windows execution is rejected before launch until native descendant "
    "ownership is proven in #1148, #1149, or #1150.",
)


@dataclass(frozen=True)
class TransportSupport:
    """The selected SDK/transport choice and its source evidence."""

    sdk: str
    transport: str
    protocol_versions: tuple[str, ...]
    platforms: tuple[str, ...]
    limitations: tuple[str, ...]
    evidence: str


SDK_TRANSPORT_CHOICE = TransportSupport(
    sdk="python-standard-library-subprocess",
    transport=DEFAULT_TRANSPORT,
    protocol_versions=SUPPORTED_PROTOCOL_VERSIONS,
    platforms=SUPPORTED_PLATFORMS,
    limitations=PLATFORM_LIMITATIONS,
    evidence=(
        "newline-delimited JSON-RPC over subprocess.Popen stdin/stdout pipes; "
        "the supported protocol is pinned to the current doctor contract "
        "2024-11-05; execution is currently POSIX-only with pre-launch "
        "rejection on unsupported platforms"
    ),
)


def supported_versions() -> dict[str, object]:
    """Return a JSON-safe copy of the supported protocol/transport contract."""

    return {
        "protocol_versions": list(SUPPORTED_PROTOCOL_VERSIONS),
        "transports": list(SUPPORTED_TRANSPORTS),
        "sdk": SDK_TRANSPORT_CHOICE.sdk,
        "transport": SDK_TRANSPORT_CHOICE.transport,
        "platforms": list(SUPPORTED_PLATFORMS),
        "runtime_platform": runtime_platform(),
        "platform_limitations": list(PLATFORM_LIMITATIONS),
        "evidence": SDK_TRANSPORT_CHOICE.evidence,
    }


def validate_support(protocol_version: str, transport: str) -> None:
    if protocol_version not in SUPPORTED_PROTOCOL_VERSIONS:
        raise ValueError(f"unsupported MCP protocol version: {protocol_version}")
    if transport not in SUPPORTED_TRANSPORTS:
        raise ValueError(f"unsupported MCP transport: {transport}")


def runtime_platform() -> str:
    """Return the stable platform label used by the support contract."""

    return "posix" if os.name == "posix" else os.name


def validate_platform(platform: str | None = None) -> None:
    """Reject execution where descendant ownership is not yet proven."""

    selected = runtime_platform() if platform is None else platform
    if selected not in SUPPORTED_PLATFORMS:
        raise ValueError(f"unsupported execution platform: {selected}")
