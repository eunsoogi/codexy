"""Packaged agent catalogs and their bounded registration parser."""

from __future__ import annotations

import json
import re
from pathlib import Path


CATALOGS = {
    "core": """# Codexy packaged-agent discovery/registration contract. Validators and the
# registration script load agent_files from this catalog; native Codex agent
# use is through marker-owned standalone files under the Codex home agents directory.
version = "0.1.0"
catalog_kind = "plugin-packaged-specialist-agent-files"
native_custom_agent_registration = "codex-home-standalone-agent-projection"
native_custom_agent_projection = "managed-codexy-subdirectory"
agent_files = [
  "codexy-architect.toml",
  "codexy-cartographer.toml",
  "codexy-auditor.toml",
  "codexy-shipwright.toml",
  "codexy-inspector.toml",
  "codexy-sentinel.toml",
  "codexy-warden.toml",
  "codexy-watcher.toml",
]
""",
    "github": """version = "0.1.0"
catalog_kind = "plugin-packaged-specialist-agent-files"
agent_files = ["codexy-weaver.toml"]
""",
}


def _catalog_agent_files(catalog: str) -> tuple[str, ...]:
    match = re.search(r"(?ms)^\s*agent_files\s*=\s*\[(.*?)\]", catalog)
    if match is None:
        raise ValueError("agent catalog must define agent_files")
    names = tuple(
        json.loads(f'"{value}"')
        for value in re.findall(r'"((?:\\.|[^"\\])*)"', match.group(1))
    )
    if (
        not names
        or any(
            not name.endswith(".toml")
            or Path(name).name != name
            or name.startswith(".")
            for name in names
        )
        or len(names) != len(set(names))
    ):
        raise ValueError("agent catalog has invalid or duplicate agent_files")
    return names
