"""Config conflict detection for the Codexy standalone-agent bridge."""

import re


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


def diagnostic_lines(config: str, agents_root) -> list[str]:
    managed_count = sum(
        path.read_text(encoding="utf-8").startswith("# CODEXY MANAGED AGENT\n")
        for path in agents_root.glob("*.toml")
    ) if agents_root.is_dir() else 0
    discovery = (
        f"PASS ({managed_count} marker-owned standalone agents)"
        if managed_count == 12
        else f"FAIL ({managed_count} marker-owned standalone agents; expected 12)"
    )
    namespace = config_value(config, "tool_namespace") or "default/unobserved"
    metadata = config_value(config, "hide_spawn_agent_metadata")
    visible = "true" if metadata == "false" else "unconfirmed"
    return [
        f"A role-discovery: {discovery}",
        f"B tool-schema: CONFIGURED (namespace={namespace}, agent_type-visible={visible}); "
        "fresh-task schema observation is still required",
        "C fork-turns: explicit agent_type requires none or a positive integer; all is incompatible",
    ]


def config_value(config: str, key: str) -> str | None:
    match = re.search(rf"(?m)^\s*{re.escape(key)}\s*=\s*(?:\"([^\"]+)\"|(true|false))\s*$", config)
    return next((group for group in match.groups() if group), None) if match else None
