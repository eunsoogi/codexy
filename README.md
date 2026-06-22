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

Install Codexy through your Codex plugin marketplace. If this repository is not
already registered as a marketplace source, add it first:

```sh
codex plugin marketplace add eunsoogi/codexy --ref main
```

Then install the plugin:

```sh
codex plugin add codexy@codexy
```

Verify that Codex can see the installed plugin and its MCP servers:

```sh
codex plugin list
codex mcp list
```

Restart Codex or open a fresh Codex session if newly installed plugin, skill, or
MCP surfaces do not appear in the active session.

## What Codexy Provides

Codexy turns Codex from a single-session coding assistant into a harness for
longer repository work. It packages instructions, specialist roles, MCP
servers, validators, and release helpers so agents can plan, implement, verify,
review, and merge work without losing the thread of ownership or evidence.

### Workflow Control

#### Scoping

- **Task classification**: names the lane type, owner, scope, and proof needed
  before repository work spreads across files or branches.
- **Atomic lanes**: helps split broad requests into issue-sized branches,
  worktrees, and PRs.

#### Ownership

- **Subthreads**: supports Codex worktree threads that own implementation for a
  specific branch or PR.
- **Subagents**: supports specialist helper and reviewer agents without
  treating them as branch-owning implementation threads.
- **Parent orchestration**: keeps routing, review follow-up, and merge
  coordination separate from child-owned patch work.

#### Progress

- **Goal and plan discipline**: keeps long-running work visible across review
  waits, verification, and handoffs.
- **Review routing**: sends feedback back to the lane that owns the change.

### Agent and Tooling Surfaces

#### Skills

- **Workflow skills**: orchestration, Git/GitHub flow, QA, debugging,
  refactoring, TDD, release engineering, and proof-driven completion.
- **Installed guidance**: ships those instructions inside the plugin so fresh
  Codex sessions can follow the same workflow.

#### Specialist Agents

- **Worker roles**: packaged agents for architecture, implementation,
  refactoring, repository mapping, and release work.
- **Reviewer roles**: a sentinel reviewer for current-diff readiness checks.

#### MCP and LSP Integration

- **Codegraph**: maps relevant files and dependencies before code changes.
- **LSP**: checks language-server registration and availability for
  language-aware edits.
- **Tool exposure checks**: distinguishes configured tools from tools that are
  actually callable in the active Codex session.

### Verification and Review Gates

#### Validators

- **Plugin checks**: validate manifest metadata, marketplace entries, MCP/LSP
  configuration, skills, agents, and release metadata.
- **Workflow checks**: validate completion handoffs, child-lane ownership
  evidence, review state, and merge-message issue references.

#### Review Readiness

- **Current-head evidence**: ties readiness to the exact file state, commit, or
  PR head being claimed.
- **Codex review gate**: treats actual Codex review output as required evidence
  and treats an `eyes` reaction as in-progress only.
- **Review thread handling**: keeps actionable comments visible until they are
  fixed or explicitly accepted.

#### GitHub and Merge Safety

- **Structured PRs**: encourages clear summaries, rationale, verification,
  follow-ups, and issue links.
- **Merge discipline**: supports squash-merge flow, branch cleanup, and
  post-merge main synchronization.

### Release and Plugin Packaging

#### Marketplace Readiness

- **Manifest and asset checks**: validates plugin metadata, marketplace
  registration, and packaged assets together.
- **Version sync**: keeps plugin, marketplace, and package metadata aligned.
- **Runtime artifacts**: checks generated archives and packaged MCP runtimes.

#### Release Workflow Support

- **Changelog helpers**: build release notes from the intended Git history.
- **Install verification**: checks that the released plugin is installable and
  that Codex can see its MCP surfaces.
