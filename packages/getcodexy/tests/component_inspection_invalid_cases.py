"""Malformed component fixtures used by health inspection tests."""

import json

CASES = (
    (
        "core",
        "agents/catalog.toml",
        'catalog_kind = "plugin-packaged-specialist-agent-files"\n',
    ),
    ("core", "hooks/hooks.json", json.dumps({"hooks": {"PreToolUse": []}})),
    (
        "devtools",
        ".mcp.json",
        json.dumps(
            {
                "lsp": {"command": "./mcp/unrelated"},
                "codegraph": {"command": "./mcp/unrelated"},
            }
        ),
    ),
    (
        "core",
        "skills/self-improvement/SKILL.md",
        "---\nname: self-improvement\n",
    ),
    ("core", "hooks/codexy-thread-delivery.sh", "#!/definitely-missing\n"),
    (
        "core",
        "skills/self-improvement/scripts/resumable_context_capsule.py",
        "",
    ),
    ("core", "skills/self-improvement/scripts/wiki_store.py", ""),
)
