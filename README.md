<p align="center">
  <img src="assets/codexy-agent-hero.png" alt="Codexy" width="100%">
</p>

<h1 align="center">Codexy</h1>

<p align="center">
  <a href="README.ko.md">Korean</a>
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-2f6f5e.svg"></a>
  <a href="https://github.com/eunsoogi/codexy/commits/main"><img alt="Last commit" src="https://img.shields.io/github/last-commit/eunsoogi/codexy.svg"></a>
  <a href="https://github.com/eunsoogi/codexy/issues"><img alt="GitHub issues" src="https://img.shields.io/github/issues/eunsoogi/codexy.svg"></a>
</p>

Codexy is a harness and loop engineering project for Codex. It helps structure, drive, observe, and verify agent work through lightweight repository-native systems.

## What's Included

Codexy packages a Codex harness as a plugin: workflow skills, specialist
roles, MCP/LSP evidence surfaces, validators, and release helpers that make
agent work easier to steer and easier to prove.

It is aimed at repository work where the agent needs more than a single prompt:

- Orchestration workflows for issue-sized lanes, child worktrees, handoffs,
  review-response routing, and long-running verification loops.
- Specialist review roles for planning, pathfinding, QA, and sentinel-style
  readiness checks before work is handed back or opened as a PR.
- MCP and LSP surfaces, including `codegraph` and `lsp`, so agents can gather
  repository evidence, inspect dependencies, and report when expected tools are
  registered but unavailable in the active session.
- Validators and release checks for plugin configuration, marketplace metadata,
  workflow contracts, touched-file size, and completion handoff evidence.
- Proof-driven GitHub and PR support for branch discipline, current-head
  review requests, status checks, unresolved review threads, and merge-ready
  evidence packets.

The root README stays high level on purpose. Executable workflow rules live in
the packaged skills so first-time users can understand the project without
needing operational setup details.

## Installation

Install Codexy from your configured Codex plugin marketplace, then confirm the plugin and MCP servers are visible:

```sh
codex plugin add codexy@codexy
codex plugin list
codex mcp list
```

Restart Codex or open a fresh Codex session after installation if newly
installed plugin tools do not appear in the active session.
