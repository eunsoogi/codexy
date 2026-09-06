"""Normalize the observed connector aliases to policy operation names."""

ALIASES = {
    "github.create_pull_request": "create_pull_request",
    "github.update_pull_request": "update_pull_request",
}


def operation(tool: str) -> str:
    return ALIASES.get(tool, tool.rsplit("github_", 1)[-1])
