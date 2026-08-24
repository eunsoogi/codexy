"""Pinned package-content integrity for public component activation."""

from __future__ import annotations

import hashlib
import json
import os
import stat
import tempfile
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

from .updater import _absolute, _validate_real_path


COMPONENT_FILES = {
    "codexy": {
        "agents/catalog.toml": "dc367f48bb175024b772bf912705e026696d636b338d29b5076a66195f6f2486",
        "agents/codexy-architect.toml": "d1bf015357395decaf94bfd3540404e896f693097141fc060f5527f6cb8c3d84",
        "agents/codexy-auditor.toml": "d95168d14419d7b206a1cf06cbdddbc813f74de7b9f77ac943a301049d9e4c0f",
        "agents/codexy-cartographer.toml": "2e6b63d15758035f99efa018d905150aa71b1b559e239a6aac3d2aec35d351e8",
        "agents/codexy-inspector.toml": "7f936f672701c475fd07a96166f058136704cfefa075231821665142c9a8fe49",
        "agents/codexy-sentinel.toml": "db0bedcc382d66c7e924b33bdf3f5668a0c83f531be4fcdc7fb04dec5c5d45a3",
        "agents/codexy-shipwright.toml": "f304a2bc88aafb100b54813eedb2b8122405a97bda24e887a0badd4cfa10a60b",
        "agents/codexy-warden.toml": "57bb2825037a5a42a0d33282ca0c800de31985c88a15d12c5d35929055c1d9ca",
        "hooks/codexy-thread-delivery.cmd": "a00b46db11963bedc90589bb644b38740491be95b9c36f962ab9f31d0e6a4797",
        "hooks/codexy-thread-delivery.sh": "70f8cc7c6415573cf61389b4a55dfd3b5bb6b037ce8549a50022e69306fe4146",
        "hooks/codexy-child-thread-creation.cmd": "9450762b7e1f7bbafc71bd6e0e8000ccb459baf5aa0aeee768236637e2e2197f",
        "hooks/codexy-child-thread-creation.sh": "0e83f9eee3dfcebcdb67ec1d6ea374ecebc5d0386a5ff4429277c64b2689b0ad",
        "hooks/codexy-child-thread-creation.py": "6f0de7208940cad5a5250422f817c778343b2b9f523af800e2b472fb0ec7471a",
        "hooks/codexy_policy/child_thread_creation.py": "9ba39931e3b459e2e58dc42cb007690aebe6e90ccb8f98969c47ab45a4c20f9d",
        "hooks/codexy_policy/envelope.py": "57256bf03570230a792fed85d844fa5715bad7c7f3ee213fe1f8ddace314330a",
        "skills/wiki/SKILL.md": "f7036d57346d8556d8b2e70cbe50a2a8dee9f87f8a8d3d630a41523a28ca8c76",
        "skills/orchestration/scripts/agent_registration_blocks.py": "d9fee4e722e6595a29aa038d3db1404f134763c80df618593f82ecc54089069b",
        "skills/orchestration/scripts/agent_registration_fs.py": "c5f1952770d4c83d662a719d24a7d30da7a266c105f9b981b99d730a8c03298e",
        "skills/orchestration/scripts/agent_registration_lifecycle.py": "9b1762d6fa066ac118c04ca61e6181997b84bf7e924ebf255703954f4e25e871",
        "skills/orchestration/scripts/agent_registration_support.py": "6aeae4d9107de34d9b79cb4c3e8898d0129b0e1f74fa57bf0825f34dd940371f",
        "skills/orchestration/scripts/register_codexy_agents.py": "3364d7bae75c351ce89aea4cbfadb46dab6260854db76851a2f13559cd8ccd7d",
        "skills/dreaming/references/handoff-runtime.schema.json": "0ca98f5c128940d99d26a36ec8e1774a818034a6d174e7610831a454779efd73",
        "skills/dreaming/scripts/resumable-context-capsule.sh": "95c7965bc494b404fc62053bd4fbd8cc85e43dbde35f00b06fbb979d6002a17a",
        "skills/dreaming/scripts/resumable-context-capsule.cmd": "be2d4141bd24b8ab5d04ff8a7641d9f83413ae0423fcf502dcc5afd1b6e30181",
        "skills/dreaming/scripts/resumable_context_capsule.py": "b6714df64fc9e7a917db0d5aae0ebe6d31d6242fa9ebfd2bd3ed81dd95e61789",
    },
    "codexy-github": {
        "agents/catalog.toml": "a40af1007d226569b0856f8a1f64e022b473644092f355df21d9468e3107880d",
        "agents/codexy-weaver.toml": "2c88b22c48eb63400d207989e98a5919479737fba2cfb855992104217a0a2353",
        "hooks/codexy-destructive-command.cmd": "1109b2c163e5ed63034a45c0e0b11c91fe4eb31b4846f44e1cd19ede916173e8",
        "hooks/codexy-destructive-command.sh": "af171870d849cd3b74f2cf3fb360751e817b348b2856486d4ae3c8417b92273e",
        "hooks/codexy-github-admission-issue.cmd": "83f6fc109d796aad68457ea252450d512be8a778b39e1ff299447cbed43934cd",
        "hooks/codexy-github-admission-pr.cmd": "7bee2a31aca39be138599bc1cf843866b79231f90bc4aacba2bfd6afab5fb492",
        "hooks/codexy-github-admission.sh": "1f8d12fe3519f2bd04804757982f761da1cf19b300a3aacae7387d63bf6bd9bb",
        "hooks/codexy-github-workflow-context.cmd": "792a4c519822b527d673b329e560716e29574c4956dc6f07bd65d2cea0ab864a",
        "hooks/codexy-github-workflow-context.sh": "7dc65b6995e7b8d9b93bc837c218bd43be7e2d6ca32c99e0d2357093825fa307",
        "hooks/codexy-repository-github-command.cmd": "fc095f99f94e32650a6f6819314e77190d11816b68ad86b4352a97320eafeca8",
        "hooks/codexy-repository-github-command.sh": "1701419442f2689d0e7f2f046fa2e4b5d531edfd6867beeede9e1a33437a3cd2",
        "skills/git-workflow/scripts/bootstrap_codexy_github_agent.py": "49983a120fd999ffc0e47e1211ecbdcb5e94a838c8c5abe833f6f4b9fb39363f",
    },
}

MANIFEST_CONTENT_DIGESTS = {
    "codexy": "5d7ceb9bd6bcb53bed92462a868ad6ab8716105138d731153197ea0950f5102b",
    "codexy-github": "626de0d0be97ea6241d0353a92f94799b73b654b8469d0d6b7ae80a88e41b197",
}


def verify_component(
    root: Path, name: str, version: str | None = None
) -> dict[Path, bytes]:
    try:
        expected = COMPONENT_FILES[name]
    except KeyError as error:
        raise ValueError(f"unknown component integrity identity: {name}") from error
    root = _absolute(root)
    _validate_real_path(root, require_exists=True)
    verified = {}
    for source, digest in expected.items():
        relative = Path(source)
        contents = _read_regular(root, relative)
        observed = hashlib.sha256(contents).hexdigest()
        if observed != digest:
            raise ValueError(f"component integrity mismatch: {root / relative}")
        verified[relative] = contents
    manifest = Path(".codex-plugin/plugin.json")
    manifest_contents = _read_regular(root, manifest)
    _verify_manifest(manifest_contents, name, version)
    verified[manifest] = manifest_contents
    return verified


@contextmanager
def frozen_component(
    root: Path, name: str, version: str | None = None
) -> Iterator[Path]:
    contents = verify_component(root, name, version)
    with tempfile.TemporaryDirectory(prefix=f"{name}-verified-") as temporary:
        target = Path(temporary)
        for relative, data in contents.items():
            destination = target / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            descriptor = os.open(
                destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600
            )
            with os.fdopen(descriptor, "wb") as output:
                output.write(data)
        yield target


def _verify_manifest(contents: bytes, name: str, version: str | None) -> None:
    try:
        manifest = json.loads(contents, object_pairs_hook=_unique_object)
    except (json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"component manifest is invalid JSON: {name}") from error
    if not isinstance(manifest, dict) or (
        manifest.get("name"),
        manifest.get("repository"),
    ) != (name, "https://github.com/eunsoogi/codexy"):
        raise ValueError(f"component manifest identity mismatch: {name}")
    if version is not None and manifest.get("version") != version:
        raise ValueError(f"component manifest version mismatch: {name}")
    normalized = dict(manifest, version="<VERSION>")
    observed = hashlib.sha256(
        json.dumps(normalized, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    if observed != MANIFEST_CONTENT_DIGESTS[name]:
        raise ValueError(f"component manifest content mismatch: {name}")


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _read_regular(root: Path, relative: Path) -> bytes:
    if _uses_windows_directory_fallback():
        return _read_regular_windows(root, relative)
    return _read_regular_posix(root, relative)


def _uses_windows_directory_fallback() -> bool:
    return os.name == "nt"


def _read_regular_posix(root: Path, relative: Path) -> bytes:
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    directory_flags = flags | getattr(os, "O_DIRECTORY", 0)
    descriptor = os.open(root, directory_flags)
    try:
        for part in relative.parts[:-1]:
            next_descriptor = os.open(part, directory_flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        with os.fdopen(
            os.open(relative.name, flags, dir_fd=descriptor), "rb"
        ) as source:
            metadata = os.fstat(source.fileno())
            if not stat.S_ISREG(metadata.st_mode):
                raise ValueError(
                    f"component integrity requires regular files: {root / relative}"
                )
            return source.read()
    finally:
        os.close(descriptor)


def _read_regular_windows(root: Path, relative: Path) -> bytes:
    target, _ = _windows_safe_path(root, relative)
    descriptor = os.open(target, os.O_RDONLY | getattr(os, "O_BINARY", 0))
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode):
            raise ValueError(f"component integrity requires regular files: {target}")
        _, final = _windows_safe_path(root, relative)
        if (opened.st_dev, opened.st_ino) != (final.st_dev, final.st_ino):
            raise ValueError(
                f"component integrity path changed while reading: {target}"
            )
        with os.fdopen(descriptor, "rb", closefd=False) as source:
            return source.read()
    finally:
        os.close(descriptor)


def _windows_safe_path(root: Path, relative: Path) -> tuple[Path, os.stat_result]:
    if relative.is_absolute() or any(
        part in {"", ".", ".."} for part in relative.parts
    ):
        raise ValueError(f"component integrity path is invalid: {relative}")
    current = root
    _windows_regular_path(current, directory=True)
    for part in relative.parts[:-1]:
        current /= part
        _windows_regular_path(current, directory=True)
    target = current / relative.name
    return target, _windows_regular_path(target, directory=False)


def _windows_regular_path(path: Path, directory: bool) -> os.stat_result:
    metadata = path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or _has_windows_reparse_point(metadata):
        raise ValueError(
            f"component integrity path must not traverse link or reparse point: {path}"
        )
    if stat.S_ISDIR(metadata.st_mode) != directory:
        kind = "directory" if directory else "regular file"
        raise ValueError(f"component integrity requires {kind}: {path}")
    return metadata


def _has_windows_reparse_point(metadata: os.stat_result) -> bool:
    attribute = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return bool(getattr(metadata, "st_file_attributes", 0) & attribute)
