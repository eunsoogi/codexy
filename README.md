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

Codexy is a harness plugin for Codex users who want repository work to stay
structured after the first prompt. After installation, it adds concrete Codex
surfaces for planning work, assigning ownership, gathering evidence, checking
review readiness, and preparing plugin releases. In practice, it gives Codex a
shared operating model for moving repository work from an issue to a branch, PR,
review, merge, and release without losing the current owner or the proof needed
for the next step.

### Installed Harness Surfaces

#### Workflow Skills

- **Task classification**: starts work by naming the lane type, owner, required
  evidence, first allowed action, and stop condition before the agent edits.
- **Orchestration workflow**: keeps parent sessions responsible for routing,
  status, and merge decisions while child worktree threads own their branch
  changes.
- **Git and GitHub workflow**: standardizes issue intake, branch creation, PR
  bodies, labels, review requests, squash merges, branch cleanup, and post-merge
  synchronization.
- **Proof-driven completion**: turns "done" into an evidence checklist tied to
  the current files, branch head, PR state, checks, review output, and external
  surfaces.
- **Release workflow**: guides version sync, package shape, marketplace
  metadata, archive checks, release notes, and release handoff work.

#### Repository Tooling

- **Codegraph MCP**: gives Codex a repository graph surface for finding relevant
  files, symbols, dependency neighbors, and likely validation touchpoints before
  direct file reads and patches.
- **LSP MCP**: records whether language-aware diagnostics are configured and
  callable for the active workspace, including the difference between registered
  tools and tools that are actually available in a session.
- **Packaged MCP registrations**: ships the MCP configuration with the plugin,
  so sessions can verify the same setup instead of rebuilding it by hand.

#### Specialist Roles

- **Worker roles**: provide reusable role definitions for implementation,
  refactoring, architecture, repository mapping, release preparation, and other
  focused lanes.
- **Reviewer roles**: provide repeatable current-diff review prompts for finding
  regressions, missing verification, workflow-rule violations, and readiness
  gaps.
- **Sentinel review gate**: supplies a packaged reviewer expectation for
  non-trivial lanes before PR readiness, so review evidence is attached to the
  exact branch or diff being claimed.

### Source Repository Maintenance

The source checkout also includes repository-maintenance scripts for plugin
authors and release work. These are not installed as end-user Codex command
surfaces by the marketplace plugin; use them when you are developing or
validating this repository itself.

- **Plugin configuration validator**: checks manifest metadata, marketplace
  registration, MCP entries, LSP catalog entries, skill frontmatter, agent
  definitions, and release metadata together.
- **Workflow contract validators**: check child-lane ownership claims,
  completion handoffs, dirty-state exceptions, merge-message issue references,
  and review-readiness evidence.
- **Version synchronization helper**: checks or updates plugin and marketplace
  versions as one release surface.

### Work Planning and Ownership Model

#### Task Intake

- **Task classification**: identifies whether a request is documentation,
  validation, implementation, release, review-response, or merge work before
  the agent starts editing.
- **Scope shaping**: turns broad requests into smaller lanes with a named owner,
  acceptance evidence, and a clear stop condition.
- **Issue-sized execution**: encourages changes that map cleanly to one issue,
  one branch, and one reviewable PR.

#### Thread and Agent Boundaries

- **Parent sessions**: coordinate routing, status checks, review-thread
  decisions, merge readiness, and post-merge synchronization.
- **Child worktree threads**: own implementation and review-response patches for
  a specific branch and issue-sized lane.
- **Specialist subagents**: assist with focused analysis, implementation advice,
  QA, or current-diff review, but do not become branch owners.
- **Ownership evidence**: records which surface owns the branch and which
  surfaces only assisted, so review feedback is sent to the right place.

#### Long-Running Progress

- **Goal tracking**: keeps the objective visible across rebases, review waits,
  verification runs, and handoffs.
- **Plan tracking**: breaks a lane into explicit pending, active, and completed
  steps.
- **Handoff discipline**: requires the next owner to receive the branch, head
  commit, evidence, blocker, and stop condition instead of vague status prose.

### Verification and Review Gates

#### Repository Validators

- **Plugin configuration checks**: validate manifest metadata, marketplace
  registration, MCP server entries, LSP catalog entries, skills, agents, and
  release metadata.
- **Workflow contract checks**: validate child-lane ownership claims,
  completion handoffs, dirty-state exceptions, review-readiness claims, and
  merge-message issue references.
- **Documentation gates**: keep documentation-only changes behind at least
  whitespace, file-existence, and touched-surface checks.

#### Review Readiness

- **Current-head proof**: ties readiness to the exact commit or PR head being
  claimed, not to stale review output from an older diff.
- **Codex review gate**: requires substantive Codex review evidence and treats
  an `eyes` reaction as a review-in-progress signal, not a merge approval.
- **Thread resolution checks**: keeps actionable, non-outdated review comments
  visible until they are fixed, accepted, or explicitly explained.

#### GitHub Safety

- **Structured PR flow**: preserves summaries, rationale, verification evidence,
  follow-ups, and issue links.
- **Merge safeguards**: supports current-head matching, squash-merge discipline,
  branch cleanup, and post-merge `main` synchronization.
- **Stop-condition reporting**: reports the exact blocker when a PR cannot be
  merged instead of treating an open PR as finished work.

### Plugin Packaging and Release Support

#### Marketplace Readiness

- **Manifest validation**: checks the public plugin identity, description,
  assets, runtime entries, and install-facing metadata together.
- **Marketplace synchronization**: keeps marketplace registration aligned with
  the packaged plugin version and metadata.
- **Asset checks**: verifies that repository-level and plugin-local visuals
  referenced by the manifest are present.

#### Release Engineering

- **Version synchronization**: checks or updates plugin, package, and
  marketplace versions as one release surface.
- **Archive and runtime checks**: validates generated plugin archives and
  packaged MCP runtimes before release handoff.
- **Release-note support**: helps turn the intended Git history into concise
  release notes and verification evidence.

#### Install Verification

- **Plugin visibility checks**: verifies that Codex lists the installed plugin.
- **MCP visibility checks**: verifies that Codex lists the installed MCP
  registrations.
- **Fresh-session guidance**: makes restart or new-session checks explicit when
  newly installed plugin surfaces are not visible in the active session.
