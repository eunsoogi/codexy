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

Release packages contain native MCP runtimes for macOS ARM64, Linux x86_64,
and Windows x86_64. The Windows package gate installs Codexy under a drive-letter
path containing spaces and Unicode, then starts both MCP servers through Codex
without WSL, Git Bash, a POSIX shell, or ambient Python.

Restart Codex or open a fresh Codex session if newly installed plugin, skill, or
MCP surfaces do not appear in the active session.

Before opening Codex after first install or an official plugin update, run the
installed plugin-root bootstrap once:

```sh
codex plugin list --marketplace codexy --json | python3 -c 'import json,os,stat,sys; d=json.load(sys.stdin); xs=[p for p in d["installed"] if p.get("pluginId")=="codexy@codexy" and p.get("name")=="codexy" and p.get("marketplaceName")=="codexy" and p.get("installed") is True and p.get("enabled") is True and p.get("source",{}).get("source")=="local" and p.get("marketplaceSource")=={"sourceType":"git","source":"https://github.com/eunsoogi/codexy.git"}]; len(xs)==1 or sys.exit("expected one enabled official Codexy install"); root=os.path.normpath(xs[0]["source"]["path"]); (os.path.isabs(root) and os.path.realpath(root)==root) or sys.exit("unsafe plugin path"); manifest=json.load(open(os.path.join(root,".codex-plugin","plugin.json"))); (manifest.get("name")=="codexy" and manifest.get("repository")=="https://github.com/eunsoogi/codexy") or sys.exit("unexpected plugin identity"); path=os.path.join(root,"bootstrap-codexy-agents"); mode=os.lstat(path).st_mode; (stat.S_ISREG(mode) and not stat.S_ISLNK(mode) and os.access(path,os.X_OK)) or sys.exit("unsafe bootstrap"); os.execv(path,[path])'
```

This prepares exact specialist agents before the first task. If it reports
`RESTART_REQUIRED`, start or restart Codex once. The read-only `SessionStart`
check detects projections made stale by an official update and points back to
the same command; hooks never rewrite user state. The in-task bootstrap remains
a safety net. Codexy keeps MCP binaries out of the source plugin and bootstraps
matching runtimes from GitHub Release assets.

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
