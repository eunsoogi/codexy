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

After installation, verify that Codex can see the plugin and its MCP servers:

```sh
codex plugin list
codex mcp list
```

Restart Codex or open a fresh Codex session after installation if newly
installed skills, specialist roles, or MCP tools do not appear in the active
session.

## What Codexy Provides

Codexy is for Codex sessions that need durable workflow control: issue-sized
implementation lanes, isolated worktrees, review-response routing, verification
evidence, and PRs that should not merge until the current head is actually
reviewed.

### Orchestration and Lane Control

- **Task classification before action**: Codexy makes the agent name the lane
  type, owner, scope, required evidence, and first allowed action before it
  starts setup, edits, PR handling, or merge work. This prevents broad requests
  from turning into one tangled branch.
- **Issue-sized lane decomposition**: independent outcomes are split into
  separate branches, worktrees, and PRs. Parent orchestration stays focused on
  routing and integration, while child worktree threads own implementation.
- **Goal and plan discipline**: long-running work keeps visible goal and plan
  state, so waiting on child threads, reviews, or asynchronous tools remains an
  active workflow instead of disappearing into vague "done soon" status.
- **Parent/child ownership boundaries**: Codexy distinguishes true Codex
  worktree threads from helper agents. Branch-and-PR implementation work stays
  with the owning child thread, and review feedback is routed back to that
  owner instead of being patched casually from the parent.

### Specialist Roles and Review Gates

- **Purpose-built specialist roles**: Codexy includes roles for planning,
  architecture, implementation, refactoring, QA, release work, workflow safety,
  and repository mapping. They give agents a clearer division of labor than a
  single all-purpose assistant loop.
- **Sentinel readiness review**: non-trivial lanes end with a reviewer gate
  that checks the current diff, exact head, scope, verification output, and
  evidence before PR readiness is claimed.
- **Helper roles without ownership confusion**: specialist agents can explore,
  review, or assist inside a lane, but they do not replace the Codex worktree
  thread that owns a branch, PR, or review-response fix.
- **Review-feedback routing**: when GitHub or Codex review comments land on a
  child-owned PR, Codexy routes the feedback to the owner with the PR number,
  head SHA, comments, expected evidence, and stop condition.

### Evidence Surfaces: MCP, LSP, and Repository Exploration

- **Codegraph exploration**: the `codegraph` surface helps agents find relevant
  files, dependencies, and nearby implementation surfaces before they edit.
  Direct file reads still confirm the final context.
- **Language-aware checks**: the `lsp` surface records whether a matching
  language server is configured and usable. When a server is missing or
  unavailable, the handoff records that fact instead of pretending diagnostics
  ran.
- **Tool exposure evidence**: Codexy treats "registered" and "callable in this
  session" as different facts. If a packaged tool is expected but unavailable,
  the workflow records the mismatch as evidence.
- **Repository-native proof**: evidence is captured from local commands,
  validators, PR state, review threads, checks, and tool output that the next
  agent or maintainer can inspect.

### Validators and Proof-Driven Completion

- **Plugin configuration validation**: validators check manifest metadata,
  marketplace registration, MCP/LSP config, skills, specialist role metadata,
  and release contracts.
- **Completion-handoff checks**: Codexy can reject handoffs that claim
  completion while a PR is still open, review threads are unresolved, review
  evidence is stale, or Codex review is only acknowledged with `eyes`.
- **Child-lane ownership checks**: evidence that assigns implementation
  ownership to the wrong surface is treated as a workflow defect before PR
  readiness.
- **Touched-file size checks**: implementation and test-harness files are kept
  small enough to review unless a narrow, tracked exception exists.
- **Proof before claims**: a lane is not considered ready because tests passed
  once. The proof has to match the current files, current commit, current PR
  head, and the external surface being claimed.

### GitHub, PR, and Merge Workflow Support

- **Branch and PR discipline**: work starts from an issue-sized scope, lands on
  a topic branch, and opens a structured PR with summary, rationale, changed
  areas, verification, not-run notes, follow-ups, and the final issue link.
- **Current-head review handling**: Codex review requests are tied to the PR
  head. If new commits land, old review output is stale; an `eyes` reaction is
  only an acknowledgement that review is in progress.
- **Review thread cleanup**: actionable review comments and unresolved threads
  block merge until fixed, verified on the current head, and resolved or
  explicitly accepted as no-change by a maintainer.
- **Squash-merge safety**: merge flow preserves the PR body, validates issue
  references, uses the reviewed head, deletes branches, and verifies the main
  worktree after merge.
- **Post-merge synchronization**: after merge, Codexy expects main to be
  refreshed and the merge evidence checked before the lane is reported done.

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
