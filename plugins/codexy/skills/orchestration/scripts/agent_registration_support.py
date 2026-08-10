"""Config conflict detection for the Codexy standalone-agent bridge."""

import re

BEGIN = "# BEGIN CODEXY MANAGED AGENTS"
END = "# END CODEXY MANAGED AGENTS"
MANAGED = "# CODEXY MANAGED AGENT\n"

def find_conflicts(text: str, names: set[str]) -> set[str]:
    text = normalize_config_keys(text, names)
    found = set()
    pattern = r"^\s*(?:\[\s*(?:agents|\"agents\"|'agents')\s*\.\s*(?:\"([^\"]+)\"|'([^']+)'|([A-Za-z0-9_-]+))(?:\s*\.[^\]]+)?\s*\]\s*(?:#[^\n]*)?$|(?:agents|\"agents\"|'agents')\s*\.\s*(?:\"([^\"]+)\"|'([^']+)'|([A-Za-z0-9_-]+))\s*(?:\.|=))"
    for match in re.finditer(pattern, text, re.MULTILINE):
        found_name = next(group for group in match.groups() if group)
        if found_name in names:
            found.add(found_name)
    prefix = re.split(r"(?m)^\s*\[", text, maxsplit=1)[0]
    if re.search(r"(?m)^\s*(?:agents|\"agents\"|'agents')\s*=", prefix):
        found.update(names)
    parent = r"(?ms)^\s*\[\s*(?:agents|\"agents\"|'agents')\s*\]\s*(?:#[^\n]*)?$\n(?P<body>.*?)(?=^\s*\[|\Z)"
    child = r"^\s*(?:\"([^\"]+)\"|'([^']+)'|([A-Za-z0-9_-]+))\s*(?:\.|=)"
    for block in re.finditer(parent, text):
        for match in re.finditer(child, block.group("body"), re.MULTILINE):
            found_name = next(group for group in match.groups() if group)
            if found_name in names:
                found.add(found_name)
    return found


def normalize_config_keys(text: str, names: set[str]) -> str:
    def replace(match: re.Match[str]) -> str:
        try:
            decoded = match.group(0)[1:-1].encode().decode("unicode_escape")
        except UnicodeDecodeError:
            return match.group(0)
        return decoded if decoded == "agents" or decoded in names else match.group(0)

    return re.sub(r'"(?:\\.|[^"\\])*"', replace, text)


def diagnostic_lines(
    config: str, installed: dict[str, str], expected: dict[str, str]
) -> list[str]:
    managed = {name: text for name, text in installed.items() if text.startswith(MANAGED)}
    exact = sum(managed.get(name) == contents for name, contents in expected.items())
    discovery = (
        f"PASS ({exact} marker-owned standalone agents)"
        if exact == len(expected) and set(managed) == set(expected)
        else f"FAIL ({exact}/{len(expected)} exact packaged standalone agents; "
        f"marker-owned files={len(managed)})"
    )
    v2 = multi_agent_v2_values(config)
    namespace = (v2 or {}).get("tool_namespace", "default/unobserved")
    metadata = (v2 or {}).get("hide_spawn_agent_metadata")
    visible = "true" if metadata == "false" else "false" if metadata == "true" else "unconfirmed"
    schema = (
        f"CONFIGURED (namespace={namespace}, agent_type-visible={visible})"
        if v2 is not None
        else "UNCONFIRMED (features.multi_agent_v2 table not configured)"
    )
    return [
        f"A role-discovery: {discovery}",
        f"B tool-schema: {schema}; "
        "fresh-task schema observation is still required",
        "C fork-turns: explicit agent_type requires none or a positive integer; all is incompatible",
    ]


def multi_agent_v2_values(config: str) -> dict[str, str] | None:
    values: dict[str, str] = {}
    in_target = found = False
    multiline: str | None = None
    container_depth = 0
    for line in config.splitlines():
        before = multiline
        multiline, closed = _multiline_state(line, multiline)
        if before is not None:
            container_depth += _container_delta(line[closed:]) if closed else 0
            continue
        delta = _container_delta(line) + (_container_delta(line[closed:]) if closed else 0)
        if multiline is not None:
            container_depth += delta
            continue
        stripped = line.strip()
        table = _table_header(stripped) if container_depth == 0 else None
        if table:
            array_table, key = table
            in_target = not array_table and bool(
                re.fullmatch(r'''(?:"features"|'features'|features)\s*\.\s*(?:"multi_agent_v2"|'multi_agent_v2'|multi_agent_v2)''', key)
            )
            found = found or in_target
            continue
        container_depth += delta
        if container_depth != 0:
            continue
        if not in_target:
            continue
        namespace = re.fullmatch(
            r'tool_namespace\s*=\s*"([^"\\]*)"\s*(?:#.*)?', stripped
        )
        metadata = re.fullmatch(
            r"hide_spawn_agent_metadata\s*=\s*(true|false)\s*(?:#.*)?", stripped
        )
        if namespace:
            values["tool_namespace"] = namespace.group(1)
        elif metadata:
            values["hide_spawn_agent_metadata"] = metadata.group(1)
    return values if found else None

def _table_header(line: str) -> tuple[bool, str] | None:
    if not line.startswith("["):
        return None
    array_table = line.startswith("[[")
    opening, closing = (2 if array_table else 1, "]]" if array_table else "]")
    index = opening
    while index < len(line):
        if line[index] in ('"', "'"):
            end = _quoted_end(line, index)
            if end is None:
                return None
            index = end
        elif line.startswith(closing, index):
            key = line[opening:index].strip()
            trailing = line[index + len(closing) :].lstrip()
            if (not trailing or trailing.startswith("#")) and _valid_key_path(key):
                return array_table, key
            return None
        else:
            index += 1
    return None


def _valid_key_path(text: str) -> bool:
    index = 0
    while index < len(text):
        while index < len(text) and text[index] in " \t":
            index += 1
        if index == len(text):
            return False
        match = re.match(
            r'''(?:(?:"(?:\\.|[^"\\])*")|'[^']*'|[A-Za-z0-9_-]+)''', text[index:]
        )
        if not match:
            return False
        index += len(match.group(0))
        while index < len(text) and text[index] in " \t":
            index += 1
        if index == len(text):
            return True
        if text[index] != ".":
            return False
        index += 1
    return False


def _quoted_end(text: str, index: int) -> int | None:
    quote = text[index]
    index += 1
    while index < len(text):
        if quote == '"' and text[index] == "\\":
            index += 2
        elif text[index] == quote:
            return index + 1
        else:
            index += 1
    return None


def _container_delta(line: str) -> int:
    delta = index = 0
    while index < len(line):
        if line[index] == "#":
            break
        if any(line.startswith(quote * 3, index) for quote in ('"', "'")):
            break
        if line[index] in ('"', "'"):
            end = _quoted_end(line, index)
            if end is None:
                break
            index = end
            continue
        if line[index] in "[{":
            delta += 1
        elif line[index] in "]}":
            delta -= 1
        index += 1
    return delta


def strip_managed_block(text: str) -> tuple[str, bool]:
    lines = text.splitlines(keepends=True)
    kept: list[str] = []
    multiline: str | None = None
    in_block = False
    found = False
    for line in lines:
        marker = line.rstrip("\r\n")
        if multiline is None and marker == BEGIN:
            if in_block:
                return text, False
            in_block = True
            found = True
            continue
        if multiline is None and marker == END:
            if not in_block:
                return text, False
            in_block = False
            continue
        if not in_block:
            kept.append(line)
        multiline, _ = _multiline_state(line, multiline)
    if in_block:
        return text, False
    return "".join(kept), found


def _multiline_state(line: str, state: str | None) -> tuple[str | None, int | None]:
    index, closed = 0, None
    while index < len(line):
        if state:
            if line.startswith(state, index) and (
                state == "'''" or not _escaped(line, index)
            ):
                state = None
                index += 3
                closed = closed or index
            else:
                index += 1
            continue
        if line[index] == "#":
            break
        triple = next(
            (quote for quote in ('"""', "'''") if line.startswith(quote, index)),
            None,
        )
        if triple:
            state = triple
            index += 3
        elif line[index] in ('"', "'"):
            index = _quoted_end(line, index) or len(line)
        else:
            index += 1
    return state, closed


def _escaped(line: str, index: int) -> bool:
    slashes = 0
    while index > slashes and line[index - slashes - 1] == "\\":
        slashes += 1
    return slashes % 2 == 1
