#!/usr/bin/env python3
"""Validate the contract-free public archive projection and list its runtimes."""

import json
import sys
from pathlib import Path

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
        command = 0
        while command < len(words) and "=" in words[command]:
            if words[command].split("=", 1)[0] == "bundled_platforms":
                return True
            command += 1
        if command == len(words):
            continue
        name = words[command]
        if name == "eval" or name in {"command", "builtin"} and "eval" in words[command + 1:]:
            return True
        if name in {"declare", "export", "local", "readonly", "typeset", "unset", "read"} and any(
            word == "bundled_platforms" or word.startswith("bundled_platforms=") for word in words[command + 1:]
        ):
            return True
        if any("${bundled_platforms:=" in word for word in words):
            return True
    return False


def logical_commands(source: str) -> list[list[str]]:
    commands, words, word = [], [], []
    quote = None
    index = 0
    while index < len(source):
        character = source[index]
        if quote is not None:
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
            if word:
                words.append("".join(word))
                word = []
        elif character in ";|&()<>":
            if word:
                words.append("".join(word))
                word = []
            if words:
                commands.append(words)
                words = []
        else:
            word.append(character)
        index += 1
    if word:
        words.append("".join(word))
    if words:
        commands.append(words)
    return commands


def continues_line(line: str) -> bool:
    quote, index = None, 0
    while index < len(line):
        character = line[index]
        if quote is not None:
            if character == "\\" and quote == '"':
                if index + 1 == len(line):
                    return True
                index += 2
                continue
            if character == quote:
                quote = None
        elif character in "'\"":
            quote = character
        elif character == "\\":
            if index + 1 == len(line):
                return True
            index += 2
            continue
        index += 1
    return False


def heredoc_delimiters(line: str) -> list[tuple[str, bool]]:
    delimiters, index, quote, word_start = [], 0, None, True
    while index < len(line):
        character = line[index]
        if quote is not None:
            if character == "\\" and quote == '"':
                index += 2
                continue
            quote = None if character == quote else quote
            word_start = False
        elif character in "'\"":
            quote, word_start = character, False
        elif character == "\\":
            word_start, index = False, index + 2
            continue
        elif character == "#" and word_start:
            break
        elif line[index:index + 2] == "<<":
            index += 2
            strip_tabs = line[index:index + 1] == "-"
            index += int(strip_tabs)
            while line[index:index + 1] in (" ", "\t"):
                index += 1
            quoted = line[index:index + 1]
            if quoted in ("'", '"'):
                end = line.find(quoted, index + 1)
                if end < 0:
                    raise ValueError("unterminated heredoc delimiter")
                delimiter, index = line[index + 1:end], end + 1
            else:
                end = index
                while end < len(line) and not line[end].isspace() and line[end] not in ";|&<>()":
                    end += 1
                delimiter, index = line[index:end], end
            if not delimiter or index < 0:
                raise ValueError("invalid heredoc")
            delimiters.append((delimiter, strip_tabs))
            word_start = False
            continue
        else:
            word_start = character.isspace() or character in ";|&()<>"
        index += 1
    return delimiters


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
