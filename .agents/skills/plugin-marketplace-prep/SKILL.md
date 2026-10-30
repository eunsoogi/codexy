---
name: plugin-marketplace-prep
description: MUST use when preparing Codex plugin manifests, marketplace listings, skill bundles, install candidates, assets, metadata, validation checks, or plugin distribution readiness.
---

# Plugin Marketplace Prep

## Purpose

MUST treat plugin discovery, structure, metadata, and assets as a product
surface. A plugin is source-ready only when Codex can discover it, users can
understand it, referenced assets resolve, and checks match the repository layout
and advertised package paths.

## Workflow

1. MUST inspect the canonical layout that exists for the candidate:
   - `.agents/plugins/marketplace.json`;
   - `plugins/<plugin>/.codex-plugin/plugin.json`;
   - `plugins/<plugin>/skills/*/SKILL.md` and their `agents/openai.yaml` files;
   - optional skill `references/*`, `agents/catalog.toml`, specialist agent
     TOMLs, `.codex/lsp-client.json`, `hooks/hooks.json`, and `.mcp.json`;
   - `plugins/<plugin>/assets/*` and any app manifests.
2. MUST validate marketplace-owned metadata and paths:
   - the plugin manifest name matches its folder and its required version field
     is present; version policy and synchronized values belong to Release
     Engineering;
   - license, author, repository, homepage, and interface copy are current;
     logo, composer icon, screenshots, and other referenced assets resolve from
     the plugin root and exist;
   - the marketplace entry uses the canonical repository-relative
     `./plugins/<plugin>` source path, states installation and authentication
     policy, has a category, and preserves intentional plugin order.
3. MUST validate skill bundles and prompts: frontmatter has `name` and
   `description`, names use lowercase hyphen-case, descriptions start with
   triggering context such as `Use when`, and `agents/openai.yaml` has
   `display_name`, `short_description`, `default_prompt`, and
   `policy.allow_implicit_invocation: true`. A default prompt MUST mention
   `$skill-name` when it routes the user.
4. MUST validate each advertised architecture surface that actually exists:
   - LSP configuration agrees with its catalog and covered extensions;
   - MCP configuration contains only verified packaged or official endpoints;
   - hooks use plugin-root-relative commands, stay read-only unless explicitly
     scoped otherwise, and avoid user-state mutation paths;
   - specialist and custom-agent TOMLs parse and use Codex-compatible fields;
     when the invoking thread is the Orchestrator, they MUST NOT define a
     competing Orchestrator. Codexy registration projects marker-owned copies
     into `$CODEX_HOME/agents/codexy/`, not versioned cache paths, and MUST NOT
     treat plugin-internal `.codex/agents/*.toml` files as canonical;
   - for Codexy, the executable root entrypoints
     `plugins/codexy/bootstrap-codexy-agents` and
     `plugins/codexy/check-codexy-agents`, plus the packaged delegates
     `plugins/codexy/skills/orchestration/scripts/bootstrap-codexy-agents` and
     `plugins/codexy/skills/orchestration/scripts/register_codexy_agents.py`,
     exist as regular, non-symlink executables; the update checker MUST remain
     read-only;
   - when packaged, the Codexy GitHub agent bootstrap exists at
     `plugins/codexy-github/skills/git-workflow/scripts/bootstrap_codexy_github_agent.py`
     as an executable;
   - Codexy reviewer-agent metadata follows the selected profile: light uses no
     LLM reviewer, standard uses one `codexy-inspector` when the profile or
     change requires independent review, and strict uses one `codexy-sentinel`;
   - when Codexy advertises LSP or code exploration, its packaged configuration
     includes the matching LSP and Codegraph servers;
   - thread/worktree guidance states handoff fields, evidence, stop conditions,
     and Orchestrator verification.
5. For Codexy, MUST run `scripts/validate-plugin-config.sh --check` when present
   and `scripts/validate-plugin-config.sh --check-touched-loc --base-ref <base>`
   for all touched governed files. Every governed file MUST stay at or below 250
   lines; MUST NOT use or authorize LOC exceptions. Relevant JSON, YAML, and
   TOML configuration MUST parse. Other plugins require checks only for their
   existing supported surfaces.
6. MUST check source-tree safety for secrets, local paths, `.omo` state,
   temporary logs, and unintended dependencies. MUST NOT create package-manager
   files solely to validate a static plugin unless its release or automation
   scope requires them.
7. When a release, artifact, or release-backed install claim is involved, MUST
   use the single
   [Marketplace Release Handoff](../release-engineering/SKILL.md#marketplace-release-handoff).
   This skill supplies the marketplace source-readiness result; it MUST NOT
   repeat release-owned version, artifact, publication, or rollback checks.

## Required Output

```text
Candidate revision:
Plugin path:
Marketplace path:
Manifest checks:
Marketplace checks:
Skill checks:
Agent checks:
LSP checks:
MCP checks:
Asset checks:
Validation commands:
Risks:
```

## Gates

- MUST NOT claim marketplace readiness while a required referenced path is
  missing or its applicable parser or asset check is absent.
- MUST NOT advertise tools, MCP servers, apps, hooks, or assets that are not
  actually packaged.
- MUST NOT present a GitHub repository, source validator, or marketplace entry
  as proof of a built artifact, actual installation, or publication. Those
  claims MUST use the shared handoff and evidence for the matching surface.
- MUST NOT add a root-level plugin manifest or marketplace file when the
  canonical layout is `.agents/plugins/marketplace.json` and
  `plugins/<plugin>/.codex-plugin/plugin.json`, obsolete package scripts,
  package-only assumptions, or local absolute paths.
- Issue-owned implementation and GitHub review feedback MUST follow
  `$orchestration` and `$git-workflow`. Source, artifact, installation, and
  publication evidence MUST follow `$proof-driven-completion`.

## Failure Modes

- Confusing a GitHub repository with an installable Codex plugin.
- Letting marketplace copy drift from implemented skills.
- Letting marketplace metadata or assets drift from the current plugin source.
- Reintroducing obsolete root manifests or stale package scripts.
- Depending on a local absolute path.
