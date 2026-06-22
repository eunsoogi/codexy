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

Codexy packages workflow-focused Codex skills, MCP wrappers such as
`codegraph` and `lsp`, specialist reviewer and helper role definitions,
validators for proof gates, and release/package helper scripts. Availability
depends on the installed plugin version and the active Codex session.

## Installation

Install Codexy from your configured Codex plugin marketplace, then confirm the plugin and MCP servers are visible:

```sh
codex plugin add codexy@codexy
codex plugin list
codex mcp list
```

Restart Codex or open a fresh Codex session after installation if newly
installed plugin tools do not appear in the active session.
