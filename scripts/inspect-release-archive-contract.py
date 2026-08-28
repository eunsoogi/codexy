#!/usr/bin/env python3
"""Validate the contract-free public archive projection and list its runtimes."""

import json
import re
import sys
from importlib import import_module
from pathlib import Path

PUBLIC_PLATFORMS = ["darwin-arm64", "linux-x86_64"]
SOURCE_WRAPPER = 'bundled_platforms="darwin-arm64 linux-x86_64"'
CANDIDATE_WRAPPER = 'bundled_platforms="darwin-arm64 linux-x86_64 windows-x86_64"'
HEREDOC_PATTERN = re.compile(r"<<(?P<strip>-)?[ \t]*(?P<quote>['\"]?)(?P<delimiter>[A-Za-z_][A-Za-z0-9_]*)(?P=quote)")

def wrapper_declarations(lines: list[str], allowed: tuple[str, ...]) -> list[int]:
    declarations, heredocs, index = [], [], 0
    while index < len(lines):
        source = lines[index].rstrip("\r\n")
        if heredocs:
            delimiter, strip_tabs = heredocs[0]
            if (source.lstrip("\t") if strip_tabs else source) == delimiter:
                heredocs.pop(0)
        else:
            continued = False
            while continues_line(source):
                continued = True
                index += 1
                if index == len(lines):
                    return []
                source = source[:-1] + lines[index].rstrip("\r\n")
            try:
                heredocs.extend(heredoc_delimiters(source))
            except ValueError:
                return []
            if source in allowed and not continued:
                declarations.append(index)
            elif has_platform_mutation(source):
                return []
        index += 1
    return declarations if not heredocs else []
def has_platform_mutation(source: str) -> bool:
    for words in logical_commands(source):
        if source.lstrip().startswith("*") and ")" in source and len(words) == 1:
            continue
        command = 0
        while command < len(words) and "=" in words[command]:
            if words[command].split("=", 1)[0] == "bundled_platforms":
                return True
            command += 1
        if command == len(words):
            continue
        name = words[command]
        if name == "eval" or any(mark in name for mark in "$`"):
            return True
        if name in {"command", "builtin"}:
            target = next((word for word in words[command + 1 :] if not word.startswith("-")), None)
            if target is None or target == "eval" or any(mark in target for mark in "$`"):
                return True
        mutators = {"declare", "export", "local", "readonly", "typeset", "unset", "read"}
        if name in mutators and any(word == "bundled_platforms" or word.startswith("bundled_platforms=") for word in words[command + 1 :]):
            return True
        if any("${bundled_platforms:=" in word for word in words):
            return True
    return False
def logical_commands(source: str) -> list[list[str]]:
    commands, words, word, quote = [], [], [], None
    def flush_word() -> None:
        if word:
            words.append("".join(word))
            word.clear()

    index = 0
    while index < len(source):
        character = source[index]
        if quote:
            if character == "\\" and quote == '"' and index + 1 < len(source):
                index += 1
                word.append(source[index])
            elif character == quote:
                quote = None
            else:
                word.append(character)
        elif character in "'\"":
            quote = character
        elif character == "\\" and index + 1 < len(source):
            index += 1
            word.append(source[index])
        elif character == "#" and not word:
            break
        elif character.isspace():
            flush_word()
        elif character in ";|&()<>":
            flush_word()
            if words:
                commands.append(words)
                words = []
        else:
            word.append(character)
        index += 1
    flush_word()
    if words:
        commands.append(words)
    return commands
def continues_line(line: str) -> bool:
    return (len(line) - len(line.rstrip("\\"))) % 2 == 1
def heredoc_delimiters(line: str) -> list[tuple[str, bool]]:
    if "<<" not in line:
        return []
    matches = list(HEREDOC_PATTERN.finditer(line))
    if not matches:
        raise ValueError("invalid heredoc")
    return [(match["delimiter"], bool(match["strip"])) for match in matches]
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
    for path in contracts:
        path.unlink()
    manifest["supportedPlatforms"] = PUBLIC_PLATFORMS
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    for path, wrapper in wrappers:
        open(path, "w", encoding="utf-8", newline="").write(wrapper)

def candidate_assembly(root: Path) -> None:
    for path in wrapper_paths(root):
        text = open(path, encoding="utf-8", newline="").read()
        open(path, "w", encoding="utf-8", newline="").write(
            rewritten_wrapper(
                text, (SOURCE_WRAPPER, CANDIDATE_WRAPPER), CANDIDATE_WRAPPER
            )
        )

def print_handoff(root: Path) -> None:
    validate = import_module("handoff_runtime_contract").validate
    manifest = validate(root / "handoff-runtime.json", root)
    for platform in manifest["platforms"].values():
        print(platform["path"])

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
        if platforms not in (full, PUBLIC_PLATFORMS):
            raise SystemExit(
                "public release archive must declare supported runtime platforms"
            )
        dispatcher = root / "mcp/codexy-mcp-devtools.exe"
        if platforms == full and not dispatcher.is_file():
            raise SystemExit(
                "public Windows package requires the shared native dispatcher"
            )
        for server in ("lsp", "codegraph"):
            legacy_native = root / "mcp" / f"codexy-mcp-{server}.exe"
            if legacy_native.exists():
                raise SystemExit(
                    f"public package must not contain the legacy native {server} entrypoint"
                )
            delegate = root / "mcp" / f"codexy-mcp-{server}.cmd"
            expected = (
                f'@echo off\n"%~dp0codexy-mcp-devtools.exe" {server} %*\n'
                "exit /b %ERRORLEVEL%\n"
            ).encode()
            if platforms == full and (
                not delegate.is_file() or delegate.read_bytes() != expected
            ):
                raise SystemExit(
                    f"public Windows package requires the thin {server} delegate"
                )
        if platforms == PUBLIC_PLATFORMS:
            if (
                dispatcher.exists()
                or any((root / "runtime").glob("*-windows-x86_64.exe"))
                or any(
                    (root / "mcp" / f"codexy-mcp-{server}.{extension}").exists()
                    for server in ("lsp", "codegraph")
                    for extension in ("cmd", "exe")
                )
            ):
                raise SystemExit(
                    "dispatcher-free legacy projection must not package Windows runtime files"
                )
        for platform in platforms:
            extension = "exe" if platform == "windows-x86_64" else "bin"
            for server in ("lsp", "codegraph"):
                print(f"runtime/codexy-mcp-{server}-{platform}.{extension}")
        if (root / "handoff-runtime.json").is_file():
            print_handoff(root)
        return
    if mode in {"source-projection", "candidate-assembly"}:
        {
            "source-projection": source_projection,
            "candidate-assembly": candidate_assembly,
        }[mode](root)
        return
    if mode == "staged":
        release = json.loads((root / "runtime-release.json").read_text())
        state = release.get("state")
        platforms = release.get("platforms", {})
        expected = (
            ("darwin-arm64", "linux-x86_64")
            if state in {"legacy-public", "source-selected"}
            else ("darwin-arm64", "linux-x86_64", "windows-x86_64")
        )
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
        if state == "candidate-proven" and "classes" in release:
            print_handoff(root)
        return
    raise SystemExit("unknown archive mode")


if __name__ == "__main__":
    main()
