from __future__ import annotations

import argparse
import os
import platform as host_platform
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import NoReturn

from .cache import plugin_release, releases_match, runtime_cache_key
from .installer import executable, execute, install_git, install_package


SUPPORTED_PLATFORMS = ("darwin-arm64", "linux-x86_64")
PROTOCOL = "stdio-newline-v1"
REPOSITORY = "https://github.com/eunsoogi/codexy"


def _fail(message: str) -> NoReturn:
    print(message, file=sys.stderr)
    raise SystemExit(127)


def _notice(message: str) -> None:
    print(f"codexy runtime: {message}", file=sys.stderr)


def _host_platform() -> str:
    override = os.environ.get("CODEXY_RUNTIME_PLATFORM")
    if override:
        return override
    os_name = {"Darwin": "darwin", "Linux": "linux", "Windows": "windows"}.get(
        host_platform.system(), "unknown"
    )
    architecture = {
        "arm64": "arm64",
        "aarch64": "arm64",
        "x86_64": "x86_64",
        "amd64": "x86_64",
        "AMD64": "x86_64",
    }.get(host_platform.machine(), "unknown")
    return f"{os_name}-{architecture}"


def _absolute_env_path(name: str) -> Path | None:
    value = os.environ.get(name)
    if not value:
        return None
    path = Path(value)
    if not path.is_absolute():
        _fail(f"{name} must be absolute: {path}")
    return path


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

    @classmethod
    def load(cls, server: str, plugin_root: Path, arguments: list[str]) -> "Configuration":
        manifest = plugin_root / ".codex-plugin/plugin.json"
        try:
            release = plugin_release(manifest)
        except (OSError, ValueError) as error:
            _fail(f"codexy-mcp-{server} cannot read plugin release: {error}")
        package_path = os.environ.get("CODEXY_RUNTIME_PACKAGE_PATH", "")
        package_url_was_set = "CODEXY_RUNTIME_PACKAGE_URL" in os.environ
        artifacts_was_set = "CODEXY_RUNTIME_ARTIFACTS_API_URL" in os.environ
        package_url = os.environ.get("CODEXY_RUNTIME_PACKAGE_URL", "")
        artifacts_api = os.environ.get("CODEXY_RUNTIME_ARTIFACTS_API_URL", "")
        package_override = bool(package_path or package_url_was_set or artifacts_was_set)
        package_sha256 = os.environ.get("CODEXY_RUNTIME_PACKAGE_SHA256", "")
        if package_override and (
            len(package_sha256) != 64
            or any(character not in "0123456789abcdefABCDEF" for character in package_sha256)
        ):
            _fail("explicit runtime package source requires CODEXY_RUNTIME_PACKAGE_SHA256")
        if not package_override:
            package_url = (
                f"{REPOSITORY}/releases/download/v{release}/"
                "codexy-marketplace-plugin.tar.gz"
            )
        return cls(
            server=server,
            plugin_root=plugin_root,
            arguments=arguments,
            platform=_host_platform(),
            manifest=manifest,
            release=release,
            runtime_name=f"codexy-mcp-{server}-{_host_platform()}.bin",
            package_path=package_path,
            package_url=package_url,
            artifacts_api=artifacts_api,
            package_override=package_override,
            package_sha256=package_sha256,
            git_repository=os.environ.get("CODEXY_RUNTIME_GIT_REPOSITORY", REPOSITORY),
            git_ref=os.environ.get("CODEXY_RUNTIME_GIT_REF", ""),
            offline=os.environ.get("UV_OFFLINE", "").lower() in {"1", "true", "yes"},
            git_fallback=os.environ.get("CODEXY_RUNTIME_GIT_FALLBACK") == "1",
        )


def _cache_root(server: str) -> Path:
    explicit = _absolute_env_path("CODEXY_RUNTIME_CACHE_DIR")
    if explicit:
        return explicit
    xdg = os.environ.get("XDG_CACHE_HOME")
    home = os.environ.get("HOME")
    if not xdg and not home:
        _fail(
            f"codexy-mcp-{server} cannot bootstrap runtime without HOME, "
            "XDG_CACHE_HOME, or CODEXY_RUNTIME_CACHE_DIR"
        )
    root = Path(xdg) if xdg else Path(home or "") / ".cache"
    if not root.is_absolute():
        _fail(f"codexy-mcp-{server} runtime cache dir must be absolute: {root}")
    return root / "codexy" / "runtime"


def _execute(config: Configuration, path: Path) -> NoReturn:
    execute(
        path,
        config.arguments,
        {"CODEXY_PLUGIN_ROOT": str(config.plugin_root)},
    )


def run(config: Configuration) -> NoReturn:
    runtime_dir = _absolute_env_path("CODEXY_RUNTIME_DIR")
    if runtime_dir:
        runtime = runtime_dir / config.runtime_name
        if not executable(runtime):
            _fail(f"codexy-mcp-{config.server} runtime not found in CODEXY_RUNTIME_DIR: {runtime}")
        _execute(config, runtime)
    if config.platform not in SUPPORTED_PLATFORMS:
        _fail(
            f"codexy-mcp-{config.server} bundled runtime supports: "
            f"{' '.join(SUPPORTED_PLATFORMS)}; set CODEXY_RUNTIME_DIR for {config.platform}"
        )
    bundled = config.plugin_root / "runtime" / config.runtime_name
    if executable(bundled):
        _execute(config, bundled)
    source_components = [
        config.package_path, config.package_url, config.artifacts_api, config.package_sha256
    ]
    source = (
        "\n".join(("package-override", *source_components))
        if config.package_override
        else "\n".join(("package-default", config.package_sha256))
    )
    key = runtime_cache_key(
        manifest=config.manifest,
        package_override=config.package_override,
        identity=[config.git_repository, config.git_ref, config.platform, PROTOCOL, source, f"codexy-mcp-{config.server}"],
    )
    install_root = _cache_root(config.server) / key
    installed = install_root / "bin" / f"codexy-mcp-{config.server}"
    marker = install_root / "plugin.json"
    if executable(installed) and config.package_override:
        _execute(config, installed)
    if executable(installed) and marker.is_file() and releases_match(config.manifest, marker)[0]:
        _execute(config, installed)
    if config.offline:
        _fail(
            f"codexy-mcp-{config.server} offline mode has no cached or bundled runtime "
            f"for {config.platform}"
        )
    try:
        _notice(f"acquiring exact release package v{config.release} for {config.server}")
        install_package(config, install_root, installed)
        _execute(config, installed)
    except (OSError, RuntimeError, ValueError) as package_error:
        if config.package_override:
            _fail(f"codexy-mcp-{config.server} explicit package source failed: {package_error}")
        if not config.git_fallback:
            _fail(f"codexy-mcp-{config.server} exact release package failed: {package_error}")
        _notice(f"release package failed ({package_error}); explicit Git fallback uses {config.git_ref}")
    try:
        install_git(config, install_root, installed)
        _execute(config, installed)
    except (OSError, RuntimeError) as git_error:
        _fail(f"codexy-mcp-{config.server} pinned Git runtime failed: {git_error}")


def main() -> None:
    parser = argparse.ArgumentParser(prog="codexy-mcp-runtime")
    parser.add_argument("server", choices=("lsp", "codegraph"))
    parser.add_argument("--plugin-root", type=Path, required=True)
    parsed, arguments = parser.parse_known_args()
    arguments = arguments[1:] if arguments[:1] == ["--"] else arguments
    run(Configuration.load(parsed.server, parsed.plugin_root.resolve(), arguments))
