"""Immutable diagnostic path requirements for installed Codexy components."""

from __future__ import annotations


DIAGNOSTIC_PATHS = {
    "core": (
        "agents/catalog.toml",
        "agents/codexy-architect.toml",
        "agents/codexy-cartographer.toml",
        "agents/codexy-auditor.toml",
        "agents/codexy-shipwright.toml",
        "agents/codexy-inspector.toml",
        "agents/codexy-sentinel.toml",
        "agents/codexy-warden.toml",
        "hooks/hooks.json",
        "hooks/codexy-thread-delivery.sh",
        "hooks/codexy-thread-delivery.cmd",
    ),
    "github": (
        "agents/catalog.toml",
        "agents/codexy-weaver.toml",
        "hooks/hooks.json",
        "hooks/codexy-github-workflow-context.sh",
        "hooks/codexy-github-workflow-context.cmd",
        "hooks/codexy-github-admission.sh",
        "hooks/codexy-github-admission-issue.cmd",
        "hooks/codexy-github-admission-pr.cmd",
    ),
    "devtools": (".mcp.json", "mcp/codexy-mcp-devtools"),
}
