"""Fail-closed shell parsing for runtime wrapper platform declarations."""


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
        if case_pattern(source, words):
            continue
        command = 0
        while command < len(words) and "=" in words[command]:
            if words[command].split("=", 1)[0] == "bundled_platforms":
                return True
            command += 1
        if command == len(words):
            continue
        name = words[command]
        if dynamic_executable(name) or name == "eval":
            return True
        if name in {"command", "builtin"}:
            target = dispatch_target(words, command + 1)
            if target is None or dynamic_executable(target) or target == "eval":
                return True
        if name in {"declare", "export", "local", "readonly", "typeset", "unset", "read"} and any(
            word == "bundled_platforms" or word.startswith("bundled_platforms=") for word in words[command + 1:]
        ):
            return True
        if any("${bundled_platforms:=" in word for word in words):
            return True
    return False


def case_pattern(source: str, words: list[str]) -> bool:
    return source.lstrip().startswith("*") and ")" in source and len(words) == 1


def dynamic_executable(word: str) -> bool:
    return "$" in word or "`" in word


def dispatch_target(words: list[str], index: int) -> str | None:
    while index < len(words) and words[index].startswith("-"):
        index += 1
    return words[index] if index < len(words) else None


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
