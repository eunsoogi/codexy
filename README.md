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

Codexy is an early-stage harness and loop engineering project for Codex. It explores how agent work can be structured, driven, observed, and verified through lightweight repository-native systems.

## Installation

Install Codexy from a configured Codex plugin marketplace:

```sh
codex plugin add codexy@codexy
codex plugin list
codex mcp list
```

Marketplace installs include the Codexy MCP wrappers for `lsp` and
`codegraph`. The wrappers run bundled runtimes when present and can bootstrap
the matching Rust runtime without requiring a local source checkout.

Codexy also packages its specialist agent definitions. After installing or
updating the plugin, run the packaged registration script from the installed
Codexy plugin directory shown by `codex plugin list`:

```sh
skills/codex-orchestration/scripts/register-codexy-agents
```

Then restart Codex or start a fresh session before expecting the Codexy agent
roles to appear in `spawn_agent`.

If you prefer to have an assistant perform the install, use this prompt:

```text
Install Codexy from my configured Codex plugin marketplace. Verify it with
codex plugin list and codex mcp list. Then run Codexy's packaged
skills/codex-orchestration/scripts/register-codexy-agents script from the
installed plugin directory, restart or refresh Codex if needed, and confirm
that the Codexy MCP servers and agent roles are available.
```
