#!/usr/bin/env python3
"""Validate the contract-free public archive projection and list its runtimes."""

import json
import sys
from pathlib import Path

PUBLIC_PLATFORMS = ["darwin-arm64", "linux-x86_64"]
CANDIDATE_PLATFORMS = [*PUBLIC_PLATFORMS, "windows-x86_64"]


def projected_wrapper(text: str) -> str:
    candidate = 'bundled_platforms="darwin-arm64 linux-x86_64 windows-x86_64"'
    source = 'bundled_platforms="darwin-arm64 linux-x86_64"'
    lines = text.splitlines(keepends=True)
    declarations = wrapper_declarations(lines, candidate)
    if len(declarations) != 1:
        raise SystemExit("candidate source projection requires three-platform wrappers")
    index = declarations[0]
    lines[index] = lines[index].replace(candidate, source)
    return "".join(lines)


def wrapper_declarations(lines: list[str], candidate: str) -> list[int]:
    declarations, heredocs, index = [], [], 0
    while index < len(lines):
        source = lines[index].rstrip("\r\n")
        if heredocs:
            delimiter, strip_tabs = heredocs[0]
            if (source.lstrip("\t") if strip_tabs else source) == delimiter:
                heredocs.pop(0)
        else:
            while continues_line(source):
                index += 1
                if index == len(lines):
                    return []
                source = source[:-1] + lines[index].rstrip("\r\n")
            try:
                heredocs.extend(heredoc_delimiters(source))
            except ValueError:
                return []
            if source == candidate:
                declarations.append(index)
            elif "bundled_platforms" in shell_code(source).replace("$bundled_platforms", ""):
                return []
        index += 1
    return declarations if not heredocs else []


def shell_code(source: str) -> str:
    characters = list(source)
    quote = None
    index = 0
    while index < len(characters):
        character = characters[index]
        if quote is not None:
            characters[index] = " "
            if character == "\\" and quote == '"' and index + 1 < len(characters):
                index += 1
                characters[index] = " "
            elif character == quote:
                quote = None
        elif character in "'\"":
            quote = character
            characters[index] = " "
        elif character == "#" and (index == 0 or source[index - 1].isspace() or source[index - 1] in ";|&()<>"):
            characters[index:] = " " * (len(characters) - index)
            break
        index += 1
    return "".join(characters)


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
    if manifest.get("supportedPlatforms") != CANDIDATE_PLATFORMS:
        raise SystemExit("candidate source projection requires three-platform manifest")
    wrappers = []
    for server in ("lsp", "codegraph"):
        path = root / "mcp" / f"codexy-mcp-{server}"
        text = path.read_text()
        wrappers.append((path, projected_wrapper(text)))
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
