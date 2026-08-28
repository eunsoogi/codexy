#!/usr/bin/env python3
"""Validate the contract-free public archive projection and list its runtimes."""

import json
import re
import shlex
import sys
from importlib import import_module
from itertools import groupby
from pathlib import Path

PUBLIC_PLATFORMS = ["darwin-arm64", "linux-x86_64"]
ALL_PLATFORMS = [*PUBLIC_PLATFORMS, "windows-x86_64"]
SERVERS = ("lsp", "codegraph")
MUTATORS = set("declare export local readonly typeset unset read".split())
SOURCE_WRAPPER = 'bundled_platforms="darwin-arm64 linux-x86_64"'
CANDIDATE_WRAPPER = 'bundled_platforms="darwin-arm64 linux-x86_64 windows-x86_64"'
HEREDOC_PATTERN = re.compile(
    r"<<(?P<strip>-)?[ \t]*(?P<quote>['\"]?)(?P<delimiter>[^ \t;|&<>()]+)(?P=quote)(?=$|[ \t;|&<>()])"
)
SHELL_LITERAL_PATTERN = re.compile(r"'(?:[^']*)'|\"(?:\\.|[^\"])*\"|\\.")


def wrapper_declarations(lines: list[str], allowed: tuple[str, ...]) -> list[int]:
    declarations, heredocs, index = [], [], 0
    while index < len(lines):
        source = lines[index].rstrip("\r\n")
        if heredocs:
            delimiter, strip_tabs = heredocs[0]
            if (source.lstrip("\t") if strip_tabs else source) == delimiter:
                heredocs.pop(0)
        else:
            consumed_continuation = False
            try:
                continued, found = shell_scan(source)
                while continued:
                    consumed_continuation = True
                    index += 1
                    if index == len(lines):
                        return []
                    source = source[:-1] + lines[index].rstrip("\r\n")
                    continued, found = shell_scan(source)
            except ValueError:
                return []
            heredocs.extend(found)
            if source in allowed and consumed_continuation:
                return []
            if source in allowed:
                declarations.append(index)
            elif has_platform_mutation(source):
                return []
        index += 1
    return declarations if not heredocs else []


def has_platform_mutation(source: str) -> bool:
    for words in logical_commands(source):
        if source.lstrip().startswith("*") and ")" in source and len(words) == 1:
            continue
        command = next(
            (index for index, word in enumerate(words) if "=" not in word), len(words)
        )
        if any(w.split("=", 1)[0] == "bundled_platforms" for w in words[:command]):
            return True
        name = words[command] if command < len(words) else ""
        if name in {"command", "builtin"}:
            tail = words[command + 1 :]
            target = next((w for w in tail if not w.startswith("-")), None)
            if target in {None, "eval"} or any(m in target for m in "$`"):
                return True
        if name == "eval" or any(mark in name for mark in "$`"):
            return True
        if name in MUTATORS and any(
            word.split("=", 1)[0] == "bundled_platforms"
            for word in words[command + 1 :]
        ):
            return True
        if any("${bundled_platforms:=" in word for word in words):
            return True
    return False


def logical_commands(source: str) -> list[list[str]]:
    lexer = shlex.shlex(source, posix=True, punctuation_chars=";|&()<>")
    lexer.whitespace_split = True
    return [
        list(words)
        for separator, words in groupby(
            lexer, lambda word: word and set(word) <= set(";|&()<>")
        )
        if not separator
    ]


def shell_scan(line: str) -> tuple[bool, list[tuple[str, bool]]]:
    masked = SHELL_LITERAL_PATTERN.sub(lambda match: "_" * len(match[0]), line)
    if comment := re.search(r"(?<!\S)#", masked):
        masked = masked[: comment.start()]
    delimiters = []
    for match in re.finditer(r"<<", masked):
        heredoc = HEREDOC_PATTERN.match(line, match.start())
        if not heredoc:
            raise ValueError("invalid heredoc")
        delimiters.append((heredoc["delimiter"], bool(heredoc["strip"])))
    return bool(re.search(r"(?<!\\)(?:\\\\)*\\$", masked)), delimiters


def wrapper_paths(root: Path) -> tuple[Path, ...]:
    shared = root / "mcp/codexy-mcp-devtools"
    paths = tuple(root / "mcp" / f"codexy-mcp-{server}" for server in SERVERS)
    return (shared,) if shared.is_file() else paths


def rewritten_wrapper(text: str, allowed: tuple[str, ...], replacement: str) -> str:
    lines = text.splitlines(keepends=True)
    declarations = wrapper_declarations(lines, allowed)
    if len(declarations) != 1:
        raise SystemExit("candidate wrapper platform declaration mismatch")
    index = declarations[0]
    lines[index] = lines[index].replace(lines[index].rstrip("\r\n"), replacement)
    return "".join(lines)


def source_projection(root: Path) -> None:
    contracts = [root / n for n in ("runtime-release.json", "runtime-candidate.json")]
    if not all(path.is_file() for path in contracts):
        raise SystemExit("candidate source projection requires runtime contracts")
    manifest_path = root / ".codex-plugin/plugin.json"
    manifest = json.loads(manifest_path.read_text())
    if manifest.get("supportedPlatforms") != [*PUBLIC_PLATFORMS, "windows-x86_64"]:
        raise SystemExit("candidate source projection requires three-platform manifest")
    rewrite_wrappers(root, (CANDIDATE_WRAPPER,), SOURCE_WRAPPER)
    for path in contracts:
        path.unlink()
    manifest["supportedPlatforms"] = PUBLIC_PLATFORMS
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")


def candidate_assembly(root: Path) -> None:
    rewrite_wrappers(root, (SOURCE_WRAPPER, CANDIDATE_WRAPPER), CANDIDATE_WRAPPER)


def rewrite_wrappers(root: Path, allowed: tuple[str, ...], replacement: str) -> None:
    for path in wrapper_paths(root):
        text = open(path, encoding="utf-8", newline="").read()
        rewritten = rewritten_wrapper(text, allowed, replacement)
        open(path, "w", encoding="utf-8", newline="").write(rewritten)


def print_handoff(root: Path) -> None:
    validate = import_module("handoff_runtime_contract").validate
    manifest = validate(root / "handoff-runtime.json", root)
    for platform in manifest["platforms"].values():
        print(platform["path"])


def fail_if(condition: bool, message: str) -> None:
    condition and sys.exit(message)


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: inspect-release-archive-contract.py MODE PLUGIN_ROOT")
    mode, root = sys.argv[1], Path(sys.argv[2])
    if mode == "public-release":
        for name in ("runtime-release.json", "runtime-candidate.json"):
            fail_if(
                (root / name).exists(),
                f"public release archive must not contain {name}",
            )
        manifest = json.loads((root / ".codex-plugin/plugin.json").read_text())
        platforms = manifest.get("supportedPlatforms")
        fail_if(
            platforms not in (ALL_PLATFORMS, PUBLIC_PLATFORMS),
            "public release archive must declare supported runtime platforms",
        )
        dispatcher = root / "mcp/codexy-mcp-devtools.exe"
        fail_if(
            platforms == ALL_PLATFORMS and not dispatcher.is_file(),
            "public Windows package requires the shared native dispatcher",
        )
        for server in SERVERS:
            legacy_native = root / "mcp" / f"codexy-mcp-{server}.exe"
            fail_if(
                legacy_native.exists(),
                f"public package must not contain the legacy native {server} entrypoint",
            )
            delegate = root / "mcp" / f"codexy-mcp-{server}.cmd"
            expected = (
                f'@echo off\n"%~dp0codexy-mcp-devtools.exe" {server} %*\n'
                "exit /b %ERRORLEVEL%\n"
            ).encode()
            fail_if(
                platforms == ALL_PLATFORMS
                and (not delegate.is_file() or delegate.read_bytes() != expected),
                f"public Windows package requires the thin {server} delegate",
            )
        if platforms == PUBLIC_PLATFORMS:
            forbidden = (
                dispatcher.exists()
                or any((root / "runtime").glob("*-windows-x86_64.exe"))
                or any(
                    (root / "mcp" / f"codexy-mcp-{server}.{extension}").exists()
                    for server in SERVERS
                    for extension in ("cmd", "exe")
                )
            )
            fail_if(
                forbidden,
                "dispatcher-free legacy projection must not package Windows runtime files",
            )
        for platform in platforms:
            extension = "exe" if platform == "windows-x86_64" else "bin"
            for server in SERVERS:
                print(f"runtime/codexy-mcp-{server}-{platform}.{extension}")
        if (root / "handoff-runtime.json").is_file():
            print_handoff(root)
        return
    if mode in {"source-projection", "candidate-assembly"}:
        (source_projection if mode == "source-projection" else candidate_assembly)(root)
        return
    if mode == "staged":
        release = json.loads((root / "runtime-release.json").read_text())
        state = release.get("state")
        platforms = release.get("platforms", {})
        expected = (
            PUBLIC_PLATFORMS
            if state in {"legacy-public", "source-selected"}
            else ALL_PLATFORMS
        )
        if set(platforms) != set(expected):
            raise SystemExit("runtime release artifact contract is invalid")
        for platform in expected:
            extension = "exe" if platform == "windows-x86_64" else "bin"
            for server in SERVERS:
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
