# Agent Instructions

## Project

Codexy is a Codex harness and loop engineering repository. It packages a
Codex plugin for agent execution loops, verification harnesses, evidence
capture, workflow automation, specialist roles, and small tools that improve
Codex work quality.

## Scope

- This file governs the whole repository.
- Keep broad repository guidance in this root `AGENTS.md`.
- Add nested `AGENTS.md` files only when a subtree has stable local rules that
  should not apply elsewhere.
- If this file conflicts with a deeper `AGENTS.md`, the deeper file wins inside
  its subtree.

## Structure

```text
codexy/
|-- README.md / README.ko.md      # first-user project introductions
|-- LICENSE                       # standard MIT license
|-- assets/                       # repository-level public visuals
|-- .agents/plugins/              # marketplace metadata
|-- .github/workflows/            # repository automation
|-- plugins/codexy/               # packaged Codexy plugin
|   |-- .codex-plugin/plugin.json # plugin manifest and marketplace surface
|   |-- .codex/lsp-client.json    # Codex LSP client config
|   |-- .mcp.json                 # packaged MCP server registrations
|   |-- agents/                   # specialist agent definitions
|   |-- assets/                   # plugin-local visual assets
|   |-- lsp/                      # LSP server catalog
|   `-- skills/                   # Codexy skill instructions
`-- scripts/                      # repository validators and release helpers
```

## Where To Look

| Task | Location | Notes |
| --- | --- | --- |
| Git, issue, PR, review, merge, labels | `plugins/codexy/skills/git-workflow/SKILL.md` | Executable workflow source of truth. |
| Plugin identity and install surface | `plugins/codexy/.codex-plugin/plugin.json` | Keep marketplace-facing metadata current. |
| Marketplace registration | `.agents/plugins/marketplace.json` | Version must stay synced with the plugin manifest. |
| Version bump automation | `.github/workflows/plugin-version-bump.yml` | Uses `scripts/sync-plugin-version.py`. |
| Plugin config validation | `scripts/validate-plugin-config.py` | Covers manifest, MCP, LSP, skills, and agent contracts. |
| Version synchronization | `scripts/sync-plugin-version.py` | Checks or updates plugin and marketplace versions. |
| Specialist agents | `plugins/codexy/agents/*.toml` | One agent per file plus `catalog.toml` and `openai.yaml`. |
| Orchestration behavior | `plugins/codexy/skills/codex-orchestration/SKILL.md` | Thread, goal, todo, multi-agent, and worktree policy. |
| Review gate contract | `plugins/codexy/agents/reviewer.toml` | Required reviewer gate for non-trivial atomic lanes. |
| MCP/LSP integration | `plugins/codexy/.mcp.json`, `plugins/codexy/.codex/lsp-client.json`, `plugins/codexy/lsp/server-catalog.toml` | Keep these validator-compatible together. |
| User-facing docs | `README.md`, `README.ko.md`, `plugins/codexy/skills/**/SKILL.md` | Root README files stay concise; skills carry executable usage detail. |
| Visual assets | `assets/`, `plugins/codexy/assets/` | Keep plugin-local assets available from the manifest. |

## Documentation

- `README.md` is the concise English first-user introduction.
- `README.ko.md` is the concise Korean first-user introduction.
- Keep both README files scoped to the current implemented state of the project.
- `LICENSE` must remain the standard English MIT license text.
- Put executable Git, issue, PR, review, and merge rules in
  `plugins/codexy/skills/git-workflow/SKILL.md`, not in this file.

## Conventions

- This repository is plugin-first: user-visible behavior usually lands under
  `plugins/codexy/**`, with validators in `scripts/**`.
- Keep specialist agents as separate `plugins/codexy/agents/*.toml` files.
- Keep skill instructions under `plugins/codexy/skills/<skill>/SKILL.md`.
- Keep MCP and LSP changes aligned with `scripts/validate-plugin-config.py`.
- Prefer repository-specific guidance over generic agent advice.
- Keep instructions actionable: use `MUST` or `MUST NOT` only for hard
  requirements.

## Verification

- Run verification that covers every touched surface before pushing or opening
  a PR.
- For documentation-only changes, at minimum run `git diff --check` and file
  existence checks for changed documents.
- For structured plugin changes, run the relevant mode of
  `scripts/validate-plugin-config.py`.
- For version metadata changes, run `scripts/sync-plugin-version.py --check`.
- Tests alone do not prove completion when the requested surface is GitHub,
  plugin packaging, a CLI, a browser page, a desktop app, or another externally
  observable workflow; drive the matching surface and capture evidence.

## Style

- Prefer small, surgical changes that directly satisfy the issue.
- Do not add speculative framework, package, or workflow assumptions.
- Mention unrelated stale work instead of cleaning it up inside the current PR.
- Do not store GitHub tokens, Codex credentials, API keys, private logs, or
  local machine paths in tracked files.
