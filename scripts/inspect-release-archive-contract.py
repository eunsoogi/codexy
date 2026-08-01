#!/usr/bin/env python3
"""Validate the contract-free public archive projection and list its runtimes."""

import json
import sys
from pathlib import Path

PUBLIC_PLATFORMS = ["darwin-arm64", "linux-x86_64"]
CANDIDATE_PLATFORMS = [*PUBLIC_PLATFORMS, "windows-x86_64"]


def source_projection(root: Path) -> None:
    contracts = [root / name for name in ("runtime-release.json", "runtime-candidate.json")]
    if not all(path.is_file() for path in contracts):
        raise SystemExit("candidate source projection requires runtime contracts")
    manifest_path = root / ".codex-plugin/plugin.json"
    manifest = json.loads(manifest_path.read_text())
    if manifest.get("supportedPlatforms") != CANDIDATE_PLATFORMS:
        raise SystemExit("candidate source projection requires three-platform manifest")
    wrappers = []
    candidate = "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\""
    source = "bundled_platforms=\"darwin-arm64 linux-x86_64\""
    for server in ("lsp", "codegraph"):
        path = root / "mcp" / f"codexy-mcp-{server}"
        text = path.read_text()
        if text.count(candidate) != 1:
            raise SystemExit("candidate source projection requires three-platform wrappers")
        wrappers.append((path, text.replace(candidate, source)))
    for path in contracts:
        path.unlink()
    manifest["supportedPlatforms"] = PUBLIC_PLATFORMS
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    for path, text in wrappers:
        path.write_text(text)


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: inspect-release-archive-contract.py MODE PLUGIN_ROOT")
    mode, root = sys.argv[1], Path(sys.argv[2])
    if mode == "public-release":
        for name in ("runtime-release.json", "runtime-candidate.json"):
            if (root / name).exists():
                raise SystemExit(f"public release archive must not contain {name}")
        manifest = json.loads((root / ".codex-plugin/plugin.json").read_text())
        platforms = manifest.get("supportedPlatforms")
        expected = ["darwin-arm64", "linux-x86_64", "windows-x86_64"]
        if platforms != expected:
            raise SystemExit("public release archive must declare darwin/linux/windows")
        for platform in platforms:
            extension = "exe" if platform == "windows-x86_64" else "bin"
            for server in ("lsp", "codegraph"):
                print(f"runtime/codexy-mcp-{server}-{platform}.{extension}")
        return
    if mode == "source-projection":
        source_projection(root)
        return
    if mode == "staged":
        release = json.loads((root / "runtime-release.json").read_text())
        state = release.get("state")
        platforms = release.get("platforms", {})
        expected = ("darwin-arm64", "linux-x86_64") if state == "legacy-public" else ("darwin-arm64", "linux-x86_64", "windows-x86_64")
        if set(platforms) != set(expected):
            raise SystemExit("runtime release artifact contract is invalid")
        for platform in expected:
            extension = "exe" if platform == "windows-x86_64" else "bin"
            for server in ("lsp", "codegraph"):
                artifact = platforms[platform][server]
                path = f"runtime/codexy-mcp-{server}-{platform}.{extension}"
                if state == "candidate-proven" and artifact.get("path") != path:
                    raise SystemExit("runtime release artifact contract is invalid")
                print(path)
        return
    raise SystemExit("unknown archive mode")


if __name__ == "__main__":
    main()
