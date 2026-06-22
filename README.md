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

Codexy gives Codex a repository-work harness: a repeatable way to turn a broad
request into scoped work, assign the right owner, verify the result, and carry
the evidence through GitHub review and merge. It is meant for work where a
single answer is not enough: issue triage, branch work, PR review response,
release preparation, plugin packaging, and long-running implementation loops.

### 1. Work Planning and Ownership

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
- **Specialist roles**: assist with focused analysis, implementation advice, QA,
  or current-diff review, but do not become branch owners.
- **Ownership evidence**: records which surface owns the branch and which
  surfaces only assisted, so review feedback is sent to the right place.

#### Long-Running Progress

- **Goal tracking**: keeps the objective visible across rebases, review waits,
  verification runs, and handoffs.
- **Plan tracking**: breaks a lane into explicit pending, active, and completed
  steps.
- **Handoff discipline**: requires the next owner to receive the branch, head
  commit, evidence, blocker, and stop condition instead of vague status prose.

### 2. Verification and Review Gates

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

### 3. Repository Intelligence

#### Code and Configuration Discovery

- **Codegraph access**: gives Codex a repository graph for finding relevant
  files, symbols, dependency neighbors, and validation touchpoints before
  editing.
- **Direct-read discipline**: pairs graph results with exact file reads, so
  patches are based on the current repository state rather than guessed
  structure.
- **Touched-surface awareness**: keeps validation focused on the files and
  contracts changed by the active lane.

#### Language and Tooling Visibility

- **LSP status checks**: records whether language diagnostics are registered,
  callable, and usable in the active workspace.
- **MCP visibility checks**: distinguishes configured MCP servers from tools
  that are actually exposed to the current Codex session.
- **Exposure-mismatch handling**: treats missing expected tools as a workflow
  defect to capture and route, not as a silent fallback.

### 4. Specialist Role Pack

#### Work Roles

- **Repository mapping**: helps locate affected files, ownership boundaries,
  nearby tests, and likely validation surfaces.
- **Implementation and refactoring**: supports focused code, documentation,
  validator, and workflow-rule changes inside an issue-sized lane.
- **Release preparation**: supports manifest, marketplace, version, archive,
  and release-note work.

#### Review Roles

- **Current-diff review**: checks the active branch or diff for regressions,
  missing verification, stale evidence, and workflow-rule violations.
- **Sentinel gate**: provides the packaged final reviewer expectation for
  non-trivial lanes before PR readiness.
- **Review-feedback routing**: sends actionable PR feedback back to the lane
  owner instead of letting another surface patch over ownership boundaries.

### 5. Plugin Packaging and Release Support

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

## Repository Contributor Tools

The source checkout also includes maintenance scripts for people developing this
repository itself. These scripts are useful for CI, release preparation, and
local repository validation, but they are not presented as end-user Codex
commands by the installed marketplace plugin.

- **Plugin configuration validator**: checks manifest metadata, marketplace
  registration, MCP entries, LSP catalog entries, instruction frontmatter,
  agent definitions, and release metadata together.
- **Workflow contract validators**: check child-lane ownership claims,
  completion handoffs, dirty-state exceptions, merge-message issue references,
  and review-readiness evidence.
- **Version synchronization helper**: checks or updates plugin and marketplace
  versions as one release surface.
