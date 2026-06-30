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
| Version bump automation | `.github/workflows/plugin-version-bump.yml` | Uses `scripts/sync-plugin-version`. |
| Plugin config validation | `scripts/validate-plugin-config` | Covers manifest, MCP, LSP, skills, and agent contracts. |
| Version synchronization | `scripts/sync-plugin-version` | Checks or updates plugin and marketplace versions. |
| Specialist agents | `plugins/codexy/agents/*.toml` | One agent per file plus `catalog.toml` and `openai.yaml`. |
| Orchestration behavior | `plugins/codexy/skills/codex-orchestration/SKILL.md` | Thread, goal, todo, multi-agent, and worktree policy. |
| Review gate contract | `plugins/codexy/agents/codexy-sentinel.toml` | Required reviewer gate for non-trivial atomic lanes. |
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
- Keep MCP and LSP changes aligned with `scripts/validate-plugin-config`.
- Use Codexy codegraph MCP for repository exploration when available, then
  confirm exact files with direct reads before editing.
- Prefer repository-specific guidance over generic agent advice.
- Keep instructions actionable: use `MUST` or `MUST NOT` only for hard
  requirements.

## Dogfooding Guardrails

- Treat failures to follow governing `AGENTS.md` files and selected skills as
  dogfooding defects. Capture the evidence and fix or explicitly track the
  defect before PR readiness.
- Every discovered dogfooding defect MUST be tracked in its own separate
  GitHub issue. Do not bundle a dogfooding defect into the current feature,
  fix, review-response, or merge lane; route it through the separate issue
  unless a maintainer explicitly re-scopes the current work to that
  issue-sized lane.
- If a repo or plugin surface is expected, registered, or enabled but is not
  available in the actual Codex callable tool surface or `tool_search`, treat
  the exposure mismatch as a dogfooding defect, not as a quiet fallback. For
  example, if `codex mcp list` shows Codexy `codegraph` or `lsp` enabled but
  the tools are not callable in the session, record both surfaces as evidence.
- Every dogfood stage MUST start from a newly created clean Codex thread before
  delegation. Do not continue a dogfood stage from an inherited, stale, or
  already-used thread context; create the fresh thread first, then delegate the
  stage with its issue, branch, owner, evidence requirements, and stop
  condition.
- Before creating Codex app threads or worktrees, preflight branch refs and do
  not pass a non-existent new branch as an existing branch selector. Wait for
  pending worktree setup before declaring failure, and keep exactly one active
  owner per issue lane before retrying or reassigning.
- Dogfooding loops must not stop at an open PR when the requested outcome
  includes completion. After verification and review gates are clean, proceed
  through merge, or explicitly report the blocker that prevents merge.
- Parent/orchestrator threads must decide lane ownership before edits.
  Child-owned lanes receive implementation and review-feedback patches in the
  child branch, not in the parent workspace.

## Verification

- Run verification that covers every touched surface before pushing or opening
  a PR.
- For documentation-only changes, at minimum run `git diff --check` and file
  existence checks for changed documents.
- For structured plugin changes, run the relevant mode of
  `scripts/validate-plugin-config`.
- For version metadata changes, run `scripts/sync-plugin-version --check`.
- Tests alone do not prove completion when the requested surface is GitHub,
  plugin packaging, a CLI, a browser page, a desktop app, or another externally
  observable workflow; drive the matching surface and capture evidence.

## Style

- Prefer small, surgical changes that directly satisfy the issue.
- Do not add speculative framework, package, or workflow assumptions.
- Mention unrelated stale work instead of cleaning it up inside the current PR.
- Do not store GitHub tokens, Codex credentials, API keys, private logs, or
  local machine paths in tracked files.
