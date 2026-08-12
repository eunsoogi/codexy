#!/usr/bin/env python3
"""Validate the contract-free public archive projection and list its runtimes."""

import json
import sys
from pathlib import Path

from release_archive_contract_shell import wrapper_declarations

PUBLIC_PLATFORMS = ["darwin-arm64", "linux-x86_64"]
SOURCE_WRAPPER = 'bundled_platforms="darwin-arm64 linux-x86_64"'
CANDIDATE_WRAPPER = 'bundled_platforms="darwin-arm64 linux-x86_64 windows-x86_64"'
BATCH_INPUT = "source-projection-batch.json"
BATCH_RESET_PATHS = (
    ".codex-plugin/plugin.json",
    "mcp/codexy-mcp-devtools",
    "runtime-candidate.json",
    "runtime-release.json",
)


def wrapper_paths(root: Path) -> tuple[Path, ...]:
    shared = root / "mcp/codexy-mcp-devtools"
    if shared.is_file():
        return (shared,)
    return tuple(root / "mcp" / f"codexy-mcp-{server}" for server in ("lsp", "codegraph"))


def rewritten_wrapper(text: str, allowed: tuple[str, ...], replacement: str) -> str:
    lines = text.splitlines(keepends=True)
    declarations = wrapper_declarations(lines, allowed)
    if len(declarations) != 1:
        raise SystemExit("candidate wrapper platform declaration mismatch")
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
    for path in wrapper_paths(root):
        text = open(path, encoding="utf-8", newline="").read()
        wrappers.append((path, rewritten_wrapper(text, (CANDIDATE_WRAPPER,), SOURCE_WRAPPER)))
    for path in contracts: path.unlink()
    manifest["supportedPlatforms"] = PUBLIC_PLATFORMS
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    for path, wrapper in wrappers:
        open(path, "w", encoding="utf-8", newline="").write(wrapper)


def source_projection_batch(root: Path) -> None:
    document = batch_document(root)
    snapshots = batch_snapshots(root)
    results = []
    for case in document["cases"]:
        restore_batch_snapshot(root, snapshots)
        wrapper = wrapper_paths(root)[0]
        text = open(wrapper, encoding="utf-8", newline="").read()
        open(wrapper, "w", encoding="utf-8", newline="").write(
            f"{text}\n{case['append']}\n"
        )
        try:
            source_projection(root)
        except SystemExit as error:
            results.append({"id": case["id"], "success": False, "diagnostic": str(error)})
        else:
            results.append({"id": case["id"], "success": True, "diagnostic": None})
    restore_batch_snapshot(root, snapshots)
    if len(results) != document["expectedCaseCount"]:
        raise SystemExit("candidate source projection batch produced incomplete results")
    print(json.dumps(results, separators=(",", ":"), sort_keys=True))


def batch_document(root: Path) -> dict:
    path = root / BATCH_INPUT
    try:
        document = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit("candidate source projection batch input is invalid") from error
    if not isinstance(document, dict):
        raise SystemExit("candidate source projection batch input is invalid")
    if document.get("resetPaths") != list(BATCH_RESET_PATHS):
        raise SystemExit("candidate source projection batch reset paths are invalid")
    cases = document.get("cases")
    expected = document.get("expectedCaseCount")
    if not isinstance(cases, list) or type(expected) is not int or expected != len(cases):
        raise SystemExit("candidate source projection batch results are incomplete")
    seen = set()
    for case in cases:
        if not isinstance(case, dict):
            raise SystemExit("candidate source projection batch input is invalid")
        identifier, appended = case.get("id"), case.get("append")
        if not isinstance(identifier, str) or not identifier or not isinstance(appended, str):
            raise SystemExit("candidate source projection batch input is invalid")
        if identifier in seen:
            raise SystemExit("candidate source projection batch IDs must be unique")
        seen.add(identifier)
    return document


def batch_snapshots(root: Path) -> dict[str, bytes]:
    snapshots = {}
    for relative in BATCH_RESET_PATHS:
        path = root / relative
        if not path.is_file():
            raise SystemExit("candidate source projection batch reset material is missing")
        snapshots[relative] = path.read_bytes()
    return snapshots


def restore_batch_snapshot(root: Path, snapshots: dict[str, bytes]) -> None:
    for relative, contents in snapshots.items():
        (root / relative).write_bytes(contents)


def candidate_assembly(root: Path) -> None:
    for path in wrapper_paths(root):
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
        full = ["darwin-arm64", "linux-x86_64", "windows-x86_64"]
        legacy = ["darwin-arm64", "linux-x86_64"]
        if platforms not in (full, legacy):
            raise SystemExit("public release archive must declare supported runtime platforms")
        dispatcher = root / "mcp/codexy-mcp-devtools.exe"
        if platforms == full and not dispatcher.is_file():
            raise SystemExit("public Windows package requires the shared native dispatcher")
        for server in ("lsp", "codegraph"):
            delegate = root / "mcp" / f"codexy-mcp-{server}.cmd"
            expected = (
                f'@echo off\n"%~dp0codexy-mcp-devtools.exe" {server} %*\n'
                "exit /b %ERRORLEVEL%\n"
            ).encode()
            if platforms == full and (
                not delegate.is_file() or delegate.read_bytes() != expected
            ):
                raise SystemExit(f"public Windows package requires the thin {server} delegate")
        if platforms == legacy:
            if (dispatcher.exists() or any((root / "runtime").glob("*-windows-x86_64.exe"))
                or any((root / "mcp" / f"codexy-mcp-{server}.cmd").exists() for server in ("lsp", "codegraph"))):
                raise SystemExit("dispatcher-free legacy projection must not package Windows runtime files")
        for platform in platforms:
            extension = "exe" if platform == "windows-x86_64" else "bin"
            for server in ("lsp", "codegraph"):
                print(f"runtime/codexy-mcp-{server}-{platform}.{extension}")
        return
    if mode in {"source-projection", "source-projection-batch", "candidate-assembly"}:
        {
            "source-projection": source_projection,
            "source-projection-batch": source_projection_batch,
            "candidate-assembly": candidate_assembly,
        }[mode](root)
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
