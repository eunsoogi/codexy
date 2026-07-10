---
name: plugin-marketplace-prep
description: MUST use when preparing Codex plugin manifests, marketplace listings, skill bundles, install candidates, assets, metadata, validation checks, or plugin distribution readiness.
---

# Plugin Marketplace Prep

## Purpose

MUST treat plugin and marketplace metadata as a product surface. A plugin is ready
only when Codex can discover it, users can understand it, assets resolve, and
validation proves the packaged paths match the repository layout.

## Workflow

1. MUST inspect canonical layout:
   - `.agents/plugins/marketplace.json`,
   - `plugins/<plugin>/.codex-plugin/plugin.json`,
   - `plugins/<plugin>/skills/*/SKILL.md`,
   - `plugins/<plugin>/skills/*/agents/openai.yaml`,
   - optional `plugins/<plugin>/skills/*/references/*`,
   - optional `plugins/<plugin>/agents/catalog.toml`,
   - optional `plugins/<plugin>/agents/*.toml` specialist agent definitions,
   - optional `plugins/<plugin>/.codex/lsp-client.json`,
   - optional `plugins/<plugin>/hooks/hooks.json`,
   - `plugins/<plugin>/assets/*`,
   - optional `plugins/<plugin>/.mcp.json` or app manifests when present.
2. Validate plugin manifest:
   - `name` matches the plugin folder,
   - `version` is explicit,
   - `license`, `author`, `repository`, and `homepage` are current,
   - interface copy matches implemented behavior,
   - referenced `logo`, `composerIcon`, screenshots, or assets exist.
3. Validate marketplace entry:
   - `source.path` is relative to the repository or marketplace installation
     root, using the canonical `./plugins/<plugin>` shape for this repository,
   - `policy.installation` and `policy.authentication` are explicit,
   - `category` is present,
   - plugin order is intentional,
   - no obsolete root `.codex-plugin`, root marketplace, or package-only
     assumption is reintroduced.
4. Validate skills:
   - `SKILL.md` has `name` and `description` frontmatter,
   - names are lowercase hyphen-case,
   - descriptions start with triggering context such as `Use when`,
   - `agents/openai.yaml` has `display_name`, `short_description`,
     `default_prompt`, and `allow_implicit_invocation: true`,
   - default prompts mention `$skill-name` when they route the user.
5. Validate packaged safety:
   - no secrets,
   - no local machine paths,
   - no `.omo` state,
   - no temporary logs,
   - no missing assets.
6. Validate architecture surfaces when present:
   - LSP config and its catalog agree on server ids and covered extensions,
   - MCP config contains only verified packaged or official endpoints,
   - lifecycle hooks use plugin-root-relative commands, stay read-only unless
     explicitly scoped otherwise, and MUST NOT use user-state mutation paths,
   - `scripts/validate-plugin-config --check-touched-loc --base-ref <base>`
     passes for touched implementation and test-harness files unless the
     tracked Codexy LOC exception mechanism names the file and rationale,
   - Codexy MCP config includes packaged `lsp` and `codegraph` servers when
     the plugin advertises LSP or code exploration behavior,
   - specialist agent or custom agent TOMLs parse and MUST NOT define a child
     orchestrator when the invoking thread is the orchestrator,
   - Codexy specialist agent TOMLs use Codex custom-agent compatible fields so
     the registration bridge can project them into the stable marker-owned
     `$CODEX_HOME/agents/codexy/` discovery subtree without versioned cache paths,
   - the Codexy agent registration script exists under
     `skills/codex-orchestration/scripts/register-codexy-agents` and is
     executable,
   - Codexy reviewer agent metadata identifies it as the mandatory gate at the
     end of every non-trivial atomic work unit,
   - thread/worktree orchestration wording includes handoff fields, evidence,
     stop conditions, and parent verification,
   - child-owned PR review feedback is routed back to the owning child thread
     and revalidated there before the parent thread merges,
   - for Codexy plugin prep specifically,
     `scripts/validate-plugin-config --check` passes when that
     validator is present in the revision being prepared,
   - for other plugins, validate only the packaged surfaces that exist instead
     of requiring the full Codexy contract.
7. Validate automation and release surfaces when present:
   - GitHub Actions reference repository-root paths,
   - workflow version inputs match plugin manifest expectations,
   - marketplace validation can run without package-manager scaffolding unless
     that scaffolding is part of the release issue.
8. MUST record install/readiness evidence and unresolved risks.

## Required Output

```text
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

- MUST NOT claim marketplace readiness while any referenced path is missing.
- MUST NOT advertise tools, MCP servers, apps, hooks, or assets that are not
  actually packaged.
- MUST NOT claim LSP, MCP, specialist agent TOML, custom agent TOML, or thread/worktree
  readiness without parser evidence and the plugin config validator when it is
  available.
- MUST NOT let the parent thread silently patch child-owned plugin architecture
  feedback. MUST route review feedback to the owning child thread and MUST require that
  thread's verification evidence before marketplace readiness.
- MUST NOT add root-level plugin manifests when the canonical layout is
  `plugins/<plugin>/.codex-plugin/plugin.json`.
- MUST NOT present plugin-internal `.codex/agents/*.toml` files as canonical or
  supported. Codexy uses `plugins/<plugin>/agents/catalog.toml` and
  `plugins/<plugin>/agents/<name>.toml` for packaged specialist definitions,
  then projects marker-owned copies into `$CODEX_HOME/agents/codexy/` when
  native `spawn_agent` roles are needed.
- MUST NOT create package manager files solely to validate a static plugin unless
  the release or automation scope requires them.

## Evidence Rules

- JSON manifests MUST require parser validation.
- LSP and MCP config MUST require parser validation and, when the plugin being
  prepared is Codexy, `scripts/validate-plugin-config --check`.
- Custom agent TOMLs and specialist agent definitions MUST require parser validation and evidence
  that no separate orchestrator agent competes with the invoking thread.
- Skill bundles MUST require frontmatter and metadata validation.
- Asset references MUST require file-existence checks from the plugin root.
- Marketplace installability MUST require checking both the repo-root-relative
  marketplace entry and plugin manifest paths.

## Failure Modes

- Confusing a GitHub repository with an installable Codex plugin.
- Letting marketplace copy drift from implemented skills.
- Reintroducing obsolete root manifests or stale package scripts.
- Shipping plugin metadata that depends on a local absolute path.
