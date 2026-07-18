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

Codexy is a Codex harness plugin for repository work that needs more structure
than a single prompt. It helps developers and teams turn broad work into
owned, reviewable lanes; use the right worker or reviewer surface; and retain
the evidence needed to finish safely.

## When Codexy helps

Use Codexy when a repository task spans planning, implementation, verification,
review, and handoff—or when several agents need clear boundaries. It is built
for work that benefits from an issue-sized branch, an explicit owner, and proof
that the current change is ready.

Codexy bundles:

- workflow instructions for classifying work, setting goals, and keeping plans
  current;
- specialist role definitions for focused implementation, investigation,
  documentation, and current-diff review;
- codegraph and language-server registrations for repository discovery and
  language-aware checks; and
- validators and GitHub-oriented evidence gates for plugin configuration,
  pull-request readiness, and release work.

## Install Codexy

Install Codexy through your Codex plugin marketplace. If this repository is not
already registered as a marketplace source, add it first:

```sh
codex plugin marketplace add \
  eunsoogi/codexy \
  --ref main
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

The first Codexy workflow that needs specialist agents runs the installed
plugin's registration bootstrap. If it reports `RESTART_REQUIRED`, open one
fresh task before specialist dispatch; you do not need to locate or run a
repository setup command. Codexy keeps MCP binaries out of the source plugin and
bootstraps matching runtimes from GitHub Release assets.

## A Codexy workflow

1. **Classify the task.** Identify the lane, owner, scope, proof, and stop
   condition before editing.
2. **Run the lane deliberately.** Keep a goal and plan, use repository and
   language-aware tooling where available, and give specialist roles bounded
   responsibilities.
3. **Prove the result.** Verify the changed surface, capture current-head
   evidence, and keep pull requests behind review and merge safeguards.

This structure helps a coordinating session route work while a child worktree
thread owns its implementation branch and review-response fixes. Focused helper
and reviewer agents can assist without becoming branch owners.

## For repository maintainers

Codexy is plugin-first. Repository governance, packaging, release, and
contributor rules stay in the canonical [agent instructions](AGENTS.md),
[plugin configuration validator](scripts/validate-plugin-config), and
[release workflow](.github/workflows/plugin-version-bump.yml), rather than in
this introduction.

## License

Codexy is available under the [MIT License](LICENSE).
