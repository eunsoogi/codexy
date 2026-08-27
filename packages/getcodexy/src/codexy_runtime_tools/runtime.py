from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path
from typing import NoReturn

from .cache import runtime_cache_key
from .installer import executable, execute, install_git, install_package
from .runtime_configuration import REPOSITORY, Configuration
from .source import ExplicitRuntimeSource, RuntimeSourceIdentity


SUPPORTED_PLATFORMS = ("darwin-arm64", "linux-x86_64")
PROTOCOL = "stdio-newline-v1"


def _fail(message: str) -> NoReturn:
    print(message, file=sys.stderr)
    raise SystemExit(127)


def _notice(message: str) -> None:
    print(f"codexy runtime: {message}", file=sys.stderr)


def _manifest_identity(
    expected_manifest: Path, observed_manifest: Path
) -> tuple[bool, str]:
    try:
        expected = json.loads(expected_manifest.read_text(encoding="utf-8"))
        observed = json.loads(observed_manifest.read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return False, f"runtime package manifest identity mismatch: {error}"
    fields = ("name", "repository", "version")
    if not isinstance(expected, dict) or not isinstance(observed, dict):
        return False, "runtime package manifest identity mismatch: invalid JSON object"
    identities = tuple(
        tuple(manifest.get(field) for field in fields)
        for manifest in (expected, observed)
    )
    complete = all(
        isinstance(value, str) and value.strip()
        for identity in identities
        for value in identity
    )
    if identities[0] != identities[1] or not complete:
        return (
            False,
            "runtime package manifest identity mismatch: "
            f"expected {identities[0]}, observed {identities[1]}",
        )
    return True, ""


def _manifest_check(config: Configuration, install_root: Path) -> tuple[bool, str]:
    if config.package_override:
        return True, ""
    return _manifest_identity(config.manifest, install_root / "plugin.json")


def _marker_valid(
    identity: RuntimeSourceIdentity, marker: Path, config: Configuration, binary: Path
) -> bool:
    if not executable(binary):
        return False
    try:
        return identity.valid_marker(
            json.loads(marker.read_text()),
            platform=config.platform,
            server=config.server,
            binary=binary.read_bytes(),
        )
    except (OSError, ValueError, json.JSONDecodeError):
        return False


def _write_marker(
    identity: RuntimeSourceIdentity, marker: Path, config: Configuration, binary: Path
) -> None:
    marker_data = identity.marker(
        platform=config.platform,
        server=config.server,
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
    )
    if marker_data is not None:
        marker.write_text(json.dumps(marker_data, sort_keys=True), encoding="utf-8")


def _absolute_env_path(name: str) -> Path | None:
    value = os.environ.get(name)
    if not value:
        return None
    path = Path(value)
    if not path.is_absolute():
        _fail(f"{name} must be absolute: {path}")
    return path


def _cache_root(server: str) -> Path:
    explicit = _absolute_env_path("CODEXY_RUNTIME_CACHE_DIR")
    if explicit:
        return explicit
    xdg, home = os.environ.get("XDG_CACHE_HOME"), os.environ.get("HOME")
    if not xdg and not home:
        _fail(
            f"codexy-mcp-{server} cannot bootstrap runtime without HOME, XDG_CACHE_HOME, or CODEXY_RUNTIME_CACHE_DIR"
        )
    root = Path(xdg) if xdg else Path(home or "") / ".cache"
    if not root.is_absolute():
        _fail(f"codexy-mcp-{server} runtime cache dir must be absolute: {root}")
    return root / "codexy" / "runtime"


def _execute(config: Configuration, path: Path) -> NoReturn:
    execute(path, config.arguments, {"CODEXY_PLUGIN_ROOT": str(config.plugin_root)})


def run(config: Configuration) -> NoReturn:
    runtime_dir = _absolute_env_path("CODEXY_RUNTIME_DIR")
    if runtime_dir:
        runtime = runtime_dir / config.runtime_name
        if not executable(runtime):
            _fail(
                f"codexy-mcp-{config.server} runtime not found in CODEXY_RUNTIME_DIR: {runtime}"
            )
        _execute(config, runtime)
    if config.platform not in SUPPORTED_PLATFORMS:
        _fail(
            f"codexy-mcp-{config.server} bundled runtime supports: {' '.join(SUPPORTED_PLATFORMS)}; set CODEXY_RUNTIME_DIR for {config.platform}"
        )
    bundled = config.plugin_root / "runtime" / config.runtime_name
    if executable(bundled):
        _execute(config, bundled)
    source_identity = config.source_identity or RuntimeSourceIdentity.create(
        explicit=ExplicitRuntimeSource.select(
            requested=config.package_override,
            package_path=config.package_path,
            package_url=config.package_url,
            artifacts_api=config.artifacts_api,
            package_sha256=config.package_sha256,
        ),
        package_sha256=config.package_sha256,
        package_url=config.package_url,
        release=config.release_contract,
    )
    source = (
        "\n".join(
            (
                "package-override",
                config.package_path,
                config.package_url,
                config.artifacts_api,
                config.package_sha256,
            )
        )
        if config.package_override
        else "\n".join(("package-default", config.package_sha256))
    )
    key = source_identity.cache_key(
        platform=config.platform, server=config.server
    ) or runtime_cache_key(
        manifest=config.manifest,
        package_override=False,
        identity=[
            config.git_repository,
            config.git_ref,
            config.platform,
            PROTOCOL,
            source,
            f"codexy-mcp-{config.server}",
        ],
    )
    install_root = _cache_root(config.server) / key
    installed = install_root / "bin" / f"codexy-mcp-{config.server}"
    marker = install_root / "runtime-marker.json"
    if executable(installed):
        matches, message = _manifest_check(config, install_root)
        if not matches and config.offline:
            _fail(message)
        if _marker_valid(source_identity, marker, config, installed) and matches:
            _execute(config, installed)
        elif (
            source_identity.cache_key(platform=config.platform, server=config.server)
            is None
            and matches
        ):
            _execute(config, installed)
    if not config.offline:
        try:
            _notice(
                f"acquiring exact release package v{config.release} for {config.server}"
            )
            install_package(config, install_root, installed)
            matches, message = _manifest_check(config, install_root)
            if not matches:
                raise RuntimeError(message)
            _write_marker(source_identity, marker, config, installed)
            _execute(config, installed)
        except (OSError, RuntimeError, ValueError) as package_error:
            if config.package_override:
                _fail(
                    f"codexy-mcp-{config.server} explicit package source failed: {package_error}"
                )
            if not config.git_fallback:
                _fail(
                    f"codexy-mcp-{config.server} exact release package failed: {package_error}"
                )
            _notice(
                f"release package failed ({package_error}); explicit Git fallback uses {config.git_ref}"
            )
    elif config.package_override or not config.git_fallback:
        _fail(
            f"codexy-mcp-{config.server} offline mode has no cached or bundled runtime for {config.platform}"
        )
    try:
        git_identity = RuntimeSourceIdentity.git_fallback(
            repository=config.git_repository, commit=config.git_ref
        )
    except ValueError as error:
        _fail(f"codexy-mcp-{config.server} pinned Git runtime failed: {error}")
    git_key = git_identity.cache_key(platform=config.platform, server=config.server)
    assert git_key is not None
    git_root = _cache_root(config.server) / git_key
    git_installed = git_root / "bin" / f"codexy-mcp-{config.server}"
    git_marker = git_root / "runtime-marker.json"
    if _marker_valid(git_identity, git_marker, config, git_installed):
        _execute(config, git_installed)
    if config.offline:
        _fail(
            f"codexy-mcp-{config.server} offline mode has no cached or bundled runtime for {config.platform}"
        )
    try:
        install_git(config, git_root, git_installed)
        _write_marker(git_identity, git_marker, config, git_installed)
        _execute(config, git_installed)
    except (OSError, RuntimeError) as git_error:
        _fail(f"codexy-mcp-{config.server} pinned Git runtime failed: {git_error}")


def main() -> None:
    parser = argparse.ArgumentParser(prog="codexy-mcp-runtime")
    parser.add_argument("server", choices=("lsp", "codegraph"))
    parser.add_argument("--plugin-root", type=Path, required=True)
    parsed, arguments = parser.parse_known_args()
    arguments = arguments[1:] if arguments[:1] == ["--"] else arguments
    run(Configuration.load(parsed.server, parsed.plugin_root.resolve(), arguments))
