#!/usr/bin/env python3
"""Validate the contract-free public archive projection and list its runtimes."""

import json
import sys
from pathlib import Path


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
