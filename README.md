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
structured after the first prompt. It bundles workflow instructions, specialist
role definitions, MCP server registrations, validators, and release utilities
that make multi-step coding work easier to route, verify, review, and finish.

The marketplace package workflow runs for changes to any file under
`plugins/codexy/**`, the marketplace manifest, or the release publish contract,
as well as the runtime sources and release scripts that build the package.
Unrelated documentation, tests, and other repository paths do not trigger the
runtime package build unless they change one of those package inputs.

### Work Planning and Ownership

#### Task Intake

- **Task classification**: identifies whether a request is documentation,
  validation, implementation, release, review-response, or merge work before
  the agent starts editing.
- **Scope shaping**: turns broad requests into smaller lanes with a named owner,
  acceptance evidence, and a clear stop condition.
- **Issue-sized execution**: encourages changes that map cleanly to one issue,
  one branch, and one reviewable PR.

#### Thread and Agent Boundaries

- **Parent orchestration**: keeps the coordinating session responsible for
  routing, status checks, review-thread decisions, and merge readiness.
- **Child worktree threads**: treats Codex threads with their own worktree and
  branch as the owners of implementation work for that lane.
- **Specialist subagents**: uses helper or reviewer agents for focused
  analysis, implementation advice, or current-diff review without confusing
  them with branch-owning child threads.
- **Ownership evidence**: records which surface owns the branch and which
  surfaces only assisted, so review feedback is sent to the right place.

#### Long-Running Progress

- **Goal tracking**: keeps the objective visible across rebases, review waits,
  verification runs, and handoffs.
- **Plan tracking**: breaks a lane into explicit pending, active, and completed
  steps.
- **Handoff discipline**: requires the next owner to receive the branch, head
  commit, evidence, blocker, and stop condition instead of vague status prose.

### Repository Intelligence and Tooling

#### Code Navigation

- **Codegraph registration**: provides a repository graph surface for finding
  relevant files, symbols, dependencies, and validation touchpoints before
  patching code.
- **Direct readback expectations**: keeps graph output as discovery evidence and
  still requires exact file reads before edits.

#### Language-Aware Work

- **LSP registration**: packages language-server configuration so agents can
  check whether language-aware diagnostics are available in the active session.
- **Exposure checks**: separates configured tools from actually callable tools,
  which helps catch plugin or session setup defects.

#### Specialist Role Catalog

- **Worker roles**: defines focused roles for implementation, architecture,
  refactoring, repository mapping, and release preparation.
- **Reviewer roles**: defines current-diff review roles that look for
  regressions, missing verification, and workflow-rule violations.
- **Role consistency**: gives repeatable prompts and boundaries to roles that
  would otherwise be improvised in each session.

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
