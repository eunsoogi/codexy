"""Load immutable runtime launch configuration from plugin metadata and environment."""

from __future__ import annotations

import os
import platform as host_platform
import sys
from dataclasses import dataclass
from pathlib import Path

from .cache import plugin_release
from .contract import RuntimeRelease, load as load_runtime_release
from .source import ExplicitRuntimeSource, RuntimeSourceIdentity


REPOSITORY = "https://github.com/eunsoogi/codexy"


def host_platform_name() -> str:
    override = os.environ.get("CODEXY_RUNTIME_PLATFORM")
    if override:
        return override
    operating_system = {"Darwin": "darwin", "Linux": "linux", "Windows": "windows"}.get(
        host_platform.system(), "unknown"
    )
    architecture = {
        "arm64": "arm64",
        "aarch64": "arm64",
        "x86_64": "x86_64",
        "amd64": "x86_64",
        "AMD64": "x86_64",
    }.get(host_platform.machine(), "unknown")
    return f"{operating_system}-{architecture}"


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(127)


@dataclass(frozen=True)
class Configuration:
    server: str
    plugin_root: Path
    arguments: list[str]
    platform: str
    manifest: Path
    release: str
    runtime_name: str
    package_path: str
    package_url: str
    artifacts_api: str
    package_override: bool
    package_sha256: str
    git_repository: str
    git_ref: str
    offline: bool
    git_fallback: bool
    release_contract: RuntimeRelease | None = None
    source_identity: RuntimeSourceIdentity | None = None

    @classmethod
    def load(
        cls, server: str, plugin_root: Path, arguments: list[str]
    ) -> "Configuration":
        manifest = plugin_root / ".codex-plugin/plugin.json"
        try:
            release = plugin_release(manifest)
        except (OSError, ValueError) as error:
            fail(f"codexy-mcp-{server} cannot read plugin release: {error}")
        path_set = "CODEXY_RUNTIME_PACKAGE_PATH" in os.environ
        package_path = os.environ.get("CODEXY_RUNTIME_PACKAGE_PATH", "")
        url_set = "CODEXY_RUNTIME_PACKAGE_URL" in os.environ
        artifacts_set = "CODEXY_RUNTIME_ARTIFACTS_API_URL" in os.environ
        package_url = os.environ.get("CODEXY_RUNTIME_PACKAGE_URL", "")
        artifacts_api = os.environ.get("CODEXY_RUNTIME_ARTIFACTS_API_URL", "")
        package_sha256 = os.environ.get("CODEXY_RUNTIME_PACKAGE_SHA256", "").lower()
        try:
            explicit_source = ExplicitRuntimeSource.select(
                requested=bool(path_set or url_set or artifacts_set),
                package_path=package_path,
                package_url=package_url,
                artifacts_api=artifacts_api,
                package_sha256=package_sha256,
            )
        except ValueError as error:
            fail(str(error))
        release_path = plugin_root / "runtime-release.json"
        try:
            release_contract = (
                load_runtime_release(plugin_root) if release_path.is_file() else None
            )
        except ValueError as error:
            fail(f"codexy-mcp-{server} cannot read runtime release: {error}")
        if release_contract and explicit_source is None:
            package_url, package_sha256 = (
                release_contract.artifact.url,
                release_contract.artifact.sha256,
            )
        elif explicit_source is None:
            package_url = f"{REPOSITORY}/releases/download/v{release}/codexy-marketplace-plugin.tar.gz"
        platform = host_platform_name()
        return cls(
            server=server,
            plugin_root=plugin_root,
            arguments=arguments,
            platform=platform,
            manifest=manifest,
            release=release,
            runtime_name=f"codexy-mcp-{server}-{platform}.bin",
            package_path=package_path,
            package_url=package_url,
            artifacts_api=artifacts_api,
            package_override=explicit_source is not None,
            package_sha256=package_sha256,
            git_repository=(
                os.environ.get("CODEXY_RUNTIME_GIT_REPOSITORY", REPOSITORY)
                if not release_contract
                else release_contract.source.repository
            ),
            git_ref=(
                os.environ.get("CODEXY_RUNTIME_GIT_REF", "")
                if not release_contract
                else release_contract.source.commit
            ),
            offline=os.environ.get("UV_OFFLINE", "").lower() in {"1", "true", "yes"},
            git_fallback=os.environ.get("CODEXY_RUNTIME_GIT_FALLBACK") == "1",
            release_contract=release_contract,
            source_identity=RuntimeSourceIdentity.create(
                explicit=explicit_source,
                package_sha256=package_sha256,
                package_url=package_url,
                release=release_contract,
            ),
        )
