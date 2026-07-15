# Agent Instructions

## Project

Codexy is a Codex harness and loop engineering repository. It packages a
Codex plugin for agent execution loops, verification harnesses, evidence
capture, workflow automation, specialist roles, and small tools that improve
Codex work quality.

## Scope

- This file governs the whole repository.
- MUST keep broad repository guidance in this root `AGENTS.md`.
- MUST add nested `AGENTS.md` files only when a subtree has stable local rules that
  MUST NOT apply elsewhere.
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
| Plugin identity and install surface | `plugins/codexy/.codex-plugin/plugin.json` | MUST keep metadata current. |
| Marketplace registration | `.agents/plugins/marketplace.json` | Version MUST stay synced with the plugin manifest. |
| Version bump automation | `.github/workflows/plugin-version-bump.yml` | Uses `scripts/sync-plugin-version`. |
| Plugin config validation | `scripts/validate-plugin-config` | Covers manifest, MCP, LSP, skills, and agent contracts. |
| Version synchronization | `scripts/sync-plugin-version` | Checks or updates plugin and marketplace versions. |
| Specialist agents | `plugins/codexy/agents/*.toml` | One agent per file plus `catalog.toml` and `openai.yaml`. |
| Orchestration behavior | `plugins/codexy/skills/codex-orchestration/SKILL.md` | Thread, goal, todo, multi-agent, and worktree policy. |
| Review gate contract | `plugins/codexy/agents/codexy-sentinel.toml` | Required reviewer gate for non-trivial atomic lanes. |
| MCP/LSP integration | `plugins/codexy/.mcp.json`, `plugins/codexy/.codex/lsp-client.json`, `plugins/codexy/lsp/server-catalog.toml` | MUST keep these validator-compatible together. |
| User-facing docs | `README.md`, `README.ko.md`, `plugins/codexy/skills/**/SKILL.md` | Root README files stay concise; skills carry executable usage detail. |
| Visual assets | `assets/`, `plugins/codexy/assets/` | MUST keep plugin-local assets available from the manifest. |

## Documentation

- `README.md` is the concise English first-user introduction.
- `README.ko.md` is the concise Korean first-user introduction.
- MUST keep both README files scoped to the current implemented state of the project.
- `LICENSE` MUST remain the standard English MIT license text.
- MUST put executable Git, issue, PR, review, and merge rules in
  `plugins/codexy/skills/git-workflow/SKILL.md`, not in this file.

## Release/version-only orchestration

- MUST discover and prefer `.github/workflows/plugin-version-bump.yml` before
  dispatching a manual child lane or creating a manual branch or PR when its
  contract covers the requested version metadata. This workflow is the existing
  automation for synchronizing the version, creating the release branch and
  commit, pushing it, and opening the version-bump PR.
- MUST record why the workflow is unavailable or insufficient before using a
  manual fallback. After either route, the parent MUST still cover issue
  linkage, repository labels, full tests, package CI, packaged Sentinel,
  merge-message validation, and post-merge release/install proof.
- MUST keep executable Git and release commands in their canonical skills rather
  than duplicating them here.

## Conventions

- This repository is plugin-first: user-visible behavior usually lands under
  `plugins/codexy/**`, with validators in `scripts/**`.
- MUST keep specialist agents as separate `plugins/codexy/agents/*.toml` files.
- MUST keep skill instructions under `plugins/codexy/skills/<skill>/SKILL.md`.
- MUST keep MCP and LSP changes aligned with `scripts/validate-plugin-config`.
- MUST use Codexy codegraph MCP for repository exploration when available, then
  MUST confirm exact files with direct reads before editing.
- Prefer repository-specific guidance over generic agent advice.
- MUST keep instructions actionable by reserving `MUST` and `MUST NOT` for hard
  requirements.
- Codexy-maintained agent-facing instruction artifacts, and agent-facing
  instruction artifacts Codexy creates or updates in other projects, MUST use
  `MUST` for mandatory agent instructions and `MUST NOT` for prohibitions.

## Dogfooding Guardrails

- MUST treat failures to follow governing `AGENTS.md` files and selected skills as
  dogfooding defects. MUST capture the evidence and fix or explicitly track the
  defect before PR readiness.
- Every actionable dogfooding defect MUST be submitted through the canonical
  approved issue-intake gate before creation. A child MUST send its parent one
  machine-readable candidate receipt and receive explicit approval. The receipt
  MUST prove supported real-surface reproduction, existing-owner exclusion,
  exhaustive all-state duplicate search, thin-harness necessity, validated
  title/body, repository-valid labels, milestone, and assignee. Unsupported
  synthetic wording, phrase variants, or observations covered by an existing
  repair MUST remain handoff-only. MUST require the intake gate and explicit
  parent approval before separate tracking. Automatic issue creation MUST NOT
  be permitted.
- If a repo or plugin surface is expected, registered, or enabled but is not
  available in the actual Codex callable tool surface or `tool_search`, MUST treat
  the exposure mismatch as a dogfooding defect, not as a quiet fallback. For
  example, if `codex mcp list` shows Codexy `codegraph` or `lsp` enabled but
  the tools are not callable in the session, record both surfaces as evidence.
- Every dogfood stage MUST start from a newly created clean Codex thread before
  delegation. MUST NOT continue a dogfood stage from an inherited, stale, or
  already-used thread context; MUST create the fresh thread first, then delegate
  the stage with its issue, branch, owner, evidence requirements, and stop
  condition.
- Before creating Codex app threads or worktrees, MUST preflight branch refs and
  MUST NOT pass a non-existent new branch as an existing branch selector. MUST wait
  for pending worktree setup before declaring failure, and MUST keep exactly one
  active owner per issue lane before retrying or reassigning.
- Dogfooding loops MUST NOT stop at an open PR when the requested outcome
  includes completion. After verification and review gates are clean, proceed
  through merge, or explicitly report the blocker that prevents merge.
- Parent/orchestrator threads MUST decide lane ownership before edits.
  Child-owned lanes receive implementation and review-feedback patches in the
  child branch, not in the parent workspace.

## Verification

- MUST run verification that covers every touched surface before pushing or opening
  a PR.
- For documentation-only changes, at minimum MUST run `git diff --check` and file
  existence checks for changed documents.
- For structured plugin changes, MUST run the relevant mode of
  `scripts/validate-plugin-config`.
- For version metadata changes, MUST run `scripts/sync-plugin-version --check`.
- Tests alone MUST NOT prove completion when the requested surface is GitHub,
  plugin packaging, a CLI, a browser page, a desktop app, or another externally
  observable workflow; MUST drive the matching surface and MUST capture evidence.

## Style

- Prefer small, surgical changes that directly satisfy the issue.
- MUST NOT add speculative framework, package, or workflow assumptions.
- MUST mention unrelated stale work instead of cleaning it up inside the current PR.
- MUST NOT store GitHub tokens, Codex credentials, API keys, private logs, or
  local machine paths in tracked files.
