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

Codexy is a Codex harness packaged as a plugin for repository work that needs
more structure than a single prompt. It helps agents split broad requests into
atomic lanes, run those lanes through the right worker or reviewer surface,
capture evidence, and keep GitHub work behind verification and review gates.

## Installation

Install Codexy from your configured Codex plugin marketplace or local
marketplace entry. For source-checkout development, register or use this
repository's marketplace entry according to your Codex plugin marketplace
configuration, then install Codexy from that marketplace.

The current repository marketplace entry is registered with:

```sh
codex plugin marketplace add eunsoogi/codexy --ref main
```

Then install the plugin from that marketplace:

```sh
codex plugin add codexy@codexy
```

After installation, verify that Codex can see the plugin and MCP servers:

```sh
codex plugin list
codex mcp list
```

Restart Codex or open a fresh Codex session after installation if newly
installed plugin, skill, or MCP surfaces do not appear in the active session.

## What Codexy Provides

Codexy is for Codex sessions that need durable workflow control: scoped
implementation lanes, isolated worktrees, review-response routing, verification
evidence, and PRs that remain understandable after several agent turns.

### Orchestration and Lane Control

- **Task classification**: Codexy helps the agent name the kind of work, the
  owner, the scope, and the evidence that will prove progress before work
  spreads across files or branches.
- **Issue-sized lanes**: broad requests can be split into focused branches,
  worktrees, and PRs, so unrelated outcomes do not get bundled into one review.
- **Goal and plan state**: long-running work keeps visible progress markers for
  handoffs, review waits, verification, and follow-up steps.
- **Parent/child boundaries**: orchestration stays separate from implementation
  ownership, which makes it clearer who should patch, verify, and respond to
  review feedback.

### Specialist Roles and Review Gates

- **Purpose-built roles**: Codexy packages focused roles for planning,
  architecture, implementation, refactoring, QA, release work, workflow safety,
  and repository mapping.
- **Readiness review**: sentinel-style review gives a second pass over scope,
  evidence, and verification before a lane is handed back or presented as PR
  ready.
- **Clear helper semantics**: specialist help is treated as assistance inside a
  lane, while worktree-based implementation remains tied to the branch and PR
  that users can inspect.
- **Review response support**: review comments can be routed back into the
  lane that owns the change, preserving context for the fix and follow-up
  evidence.

### Evidence Surfaces: MCP, LSP, and Repository Exploration

- **Codegraph exploration**: `codegraph` helps agents find relevant files,
  dependencies, and nearby surfaces before they edit.
- **Language-aware checks**: `lsp` records whether a matching language server
  is configured and usable for the files under review.
- **Tool availability evidence**: Codexy distinguishes configured tools from
  tools that are actually callable in the active session.
- **Repository-native proof**: command output, validator results, PR state,
  review threads, and tool output become evidence that another agent or
  maintainer can inspect.

### Validators and Proof-Driven Completion

- **Plugin configuration validation**: validators cover manifest metadata,
  marketplace registration, MCP/LSP configuration, skills, role metadata, and
  release contracts.
- **Completion evidence checks**: handoff evidence can be checked against PR
  state, review status, and current-head review output before a lane is called
  ready.
- **Ownership evidence checks**: child-lane evidence helps catch confusion
  between orchestration, helper work, and branch-owning implementation.
- **Reviewable file sizes**: touched implementation and test-harness files can
  be checked against local size targets to keep reviews manageable.
- **Current-state proof**: Codexy emphasizes evidence that matches the current
  files, commit, PR head, and external surface being discussed.

### GitHub, PR, and Merge Workflow Support

- **Structured PRs**: Codexy encourages PRs with clear summaries, rationale,
  changed areas, verification, not-run notes, follow-ups, and issue links.
- **Current-head review awareness**: review evidence is associated with the PR
  head it reviewed, which helps users spot stale feedback after new commits.
- **Review thread visibility**: actionable comments and unresolved threads stay
  visible as part of the readiness picture.
- **Squash-merge support**: merge helpers focus on preserving PR body context,
  issue references, branch cleanup, and post-merge verification.
- **Post-merge evidence**: refreshed main state and merge-message checks help
  prove that the repository ended up where the PR said it would.

### Release and Plugin Packaging Support

- **Version synchronization**: release helpers keep plugin metadata,
  marketplace entries, and package metadata aligned.
- **Runtime artifact checks**: packaging validation covers runtime binaries,
  platform support, and generated plugin archives.
- **Changelog generation**: release tooling can build changelog text from Git
  tags while avoiding newer tags outside the release history.
- **Marketplace publication contracts**: validators check the source
  marketplace, package archive, workflow triggers, and publication expectations
  before release work is treated as ready.
- **Local install verification**: release lanes include observable install and
  MCP visibility checks so a published plugin is not considered ready only
  because files were generated.
