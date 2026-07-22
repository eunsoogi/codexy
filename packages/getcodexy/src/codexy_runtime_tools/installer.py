from __future__ import annotations

import os
import re
import shutil
import stat
import subprocess
import tempfile
from pathlib import Path
from typing import NoReturn, Protocol

from .cache import releases_match
from .package import acquire_package, unpack_runtime


class InstallConfig(Protocol):
    server: str
    manifest: Path
    runtime_name: str
    package_path: str
    package_url: str
    artifacts_api: str
    package_override: bool
    package_sha256: str
    git_repository: str
    git_ref: str
    source_identity: object


def executable(path: Path) -> bool:
    try:
        metadata = os.lstat(path)
    except FileNotFoundError:
        return False
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    return (
        stat.S_ISREG(metadata.st_mode)
        and not stat.S_ISLNK(metadata.st_mode)
        and not bool(getattr(metadata, "st_file_attributes", 0) & reparse)
        and os.access(path, os.X_OK)
    )


def execute(
    path: Path | str, arguments: list[str], environment: dict[str, str] | None = None
) -> NoReturn:
    command = str(path)
    runtime_environment = os.environ.copy()
    runtime_environment.update(environment or {})
    os.execvpe(command, [command, *arguments], runtime_environment)
    raise AssertionError("exec returned unexpectedly")


def install_package(config: InstallConfig, install_root: Path, installed: Path) -> None:
    install_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="package-", dir=install_root) as temporary:
        work = Path(temporary)
        archive = acquire_package(
            path=config.package_path,
            url=config.package_url,
            artifacts_api=config.artifacts_api,
            expected_sha256=config.package_sha256,
            work=work,
        )
        source_identity = getattr(config, "source_identity", None)
        release_contract = getattr(config, "release_contract", None)
        if source_identity is not None:
            source_identity.verify_archive(archive, platform=config.platform)
        elif release_contract is not None:
            release_contract.verify_archive(archive, platform=config.platform)
        packaged_runtime, package_manifest = unpack_runtime(
            archive=archive, work=work, runtime_name=config.runtime_name
        )
        if not config.package_override and release_contract is None:
            matches, message = releases_match(config.manifest, package_manifest)
            if not matches:
                raise RuntimeError(message)
        installed.parent.mkdir(parents=True, exist_ok=True)
        temporary_runtime = installed.with_name(f".{installed.name}.{os.getpid()}.tmp")
        shutil.copyfile(packaged_runtime, temporary_runtime)
        temporary_runtime.chmod(
            temporary_runtime.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
        )
        os.replace(temporary_runtime, installed)
        if not config.package_override:
            shutil.copyfile(package_manifest, install_root / "plugin.json")


def install_git(config: InstallConfig, install_root: Path, installed: Path) -> None:
    cargo = shutil.which("cargo")
    if not cargo:
        raise RuntimeError("cargo is unavailable for the configured Git runtime source")
    if config.git_repository != "https://github.com/eunsoogi/codexy" or not re.fullmatch(r"[0-9a-f]{40}", config.git_ref):
        raise RuntimeError("Git fallback requires the canonical repository and lowercase 40-hex commit")
    install_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="git-", dir=install_root) as temporary:
        staged_root = Path(temporary) / "root"
        staged_runtime = staged_root / "bin" / f"codexy-mcp-{config.server}"
        command = [
            cargo,
            "install",
            "--force",
            "--locked",
            "--git",
            config.git_repository,
            "--rev",
            config.git_ref,
            "--root",
            str(staged_root),
            "--bin",
            f"codexy-mcp-{config.server}",
        ]
        environment = {key: value for key, value in os.environ.items() if key not in {"GH_TOKEN", "GITHUB_TOKEN"}}
        completed = subprocess.run(command, check=False, env=environment)
        if completed.returncode or not executable(staged_runtime):
            raise RuntimeError(f"cargo install exited with status {completed.returncode}")
        installed.parent.mkdir(parents=True, exist_ok=True)
        temporary_runtime = installed.with_name(f".{installed.name}.{os.getpid()}.tmp")
        shutil.copyfile(staged_runtime, temporary_runtime)
        os.replace(temporary_runtime, installed)
