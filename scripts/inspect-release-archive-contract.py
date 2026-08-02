#!/usr/bin/env python3
"""Validate the contract-free public archive projection and list its runtimes."""

import json
import sys
from pathlib import Path

from release_archive_contract_shell import wrapper_declarations

PUBLIC_PLATFORMS = ["darwin-arm64", "linux-x86_64"]
SOURCE_WRAPPER = 'bundled_platforms="darwin-arm64 linux-x86_64"'
CANDIDATE_WRAPPER = 'bundled_platforms="darwin-arm64 linux-x86_64 windows-x86_64"'


def rewritten_wrapper(text: str, allowed: tuple[str, ...], replacement: str) -> str:
    lines = text.splitlines(keepends=True)
    declarations = wrapper_declarations(lines, allowed)
    if len(declarations) != 1:
        raise SystemExit("candidate source projection requires three-platform wrappers")
    index = declarations[0]
    lines[index] = lines[index].replace(lines[index].rstrip("\r\n"), replacement)
    return "".join(lines)


def source_projection(root: Path) -> None:
    contracts = [root / name for name in ("runtime-release.json", "runtime-candidate.json")]
    if not all(path.is_file() for path in contracts):
        raise SystemExit("candidate source projection requires runtime contracts")
    manifest_path = root / ".codex-plugin/plugin.json"
    manifest = json.loads(manifest_path.read_text())
    if manifest.get("supportedPlatforms") != [*PUBLIC_PLATFORMS, "windows-x86_64"]:
        raise SystemExit("candidate source projection requires three-platform manifest")
    wrappers = []
    for server in ("lsp", "codegraph"):
        path = root / "mcp" / f"codexy-mcp-{server}"
        text = open(path, encoding="utf-8", newline="").read()
        wrappers.append((path, rewritten_wrapper(text, (CANDIDATE_WRAPPER,), SOURCE_WRAPPER)))
    for path in contracts: path.unlink()
    manifest["supportedPlatforms"] = PUBLIC_PLATFORMS
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    for path, text in wrappers: open(path, "w", encoding="utf-8", newline="").write(text)


def candidate_assembly(root: Path) -> None:
    for server in ("lsp", "codegraph"):
        path = root / "mcp" / f"codexy-mcp-{server}"
        text = open(path, encoding="utf-8", newline="").read()
        open(path, "w", encoding="utf-8", newline="").write(rewritten_wrapper(text, (SOURCE_WRAPPER, CANDIDATE_WRAPPER), CANDIDATE_WRAPPER))


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
    if mode in {"source-projection", "candidate-assembly"}:
        {"source-projection": source_projection, "candidate-assembly": candidate_assembly}[mode](root)
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
