---
name: plugin-marketplace-prep
description: Use when preparing Codex plugin manifests, marketplace listings, skill bundles, install candidates, assets, metadata, validation checks, or plugin distribution readiness.
---

# Plugin Marketplace Prep

## Purpose

Treat plugin and marketplace metadata as a product surface. A plugin is ready
only when Codex can discover it, users can understand it, assets resolve, and
validation proves the packaged paths match the repository layout.

## Workflow

1. Inspect canonical layout:
   - `.agents/plugins/marketplace.json`,
   - `plugins/<plugin>/.codex-plugin/plugin.json`,
   - `plugins/<plugin>/skills/*/SKILL.md`,
   - `plugins/<plugin>/skills/*/agents/openai.yaml`,
   - optional `plugins/<plugin>/agents/catalog.toml`,
   - optional `plugins/<plugin>/agents/roles/*.toml`,
   - optional `plugins/<plugin>/.codex/lsp-client.json`,
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
   - role metadata or custom agent TOMLs parse and do not define a child
     orchestrator when the invoking thread is the orchestrator,
   - thread/worktree orchestration wording includes handoff fields, evidence,
     stop conditions, and parent verification,
   - child-owned PR review feedback is routed back to the owning child thread
     and revalidated there before the parent thread merges,
   - for Codexy plugin prep specifically,
     `python3 scripts/validate-plugin-config.py --check` passes when that
     validator is present in the revision being prepared,
   - for other plugins, validate only the packaged surfaces that exist instead
     of requiring the full Codexy contract.
7. Validate automation and release surfaces when present:
   - GitHub Actions reference repository-root paths,
   - workflow version inputs match plugin manifest expectations,
   - marketplace validation can run without package-manager scaffolding unless
     that scaffolding is part of the release issue.
8. Record install/readiness evidence and unresolved risks.

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

- Do not claim marketplace readiness while any referenced path is missing.
- Do not advertise tools, MCP servers, apps, hooks, or assets that are not
  actually packaged.
- Do not claim LSP, MCP, role metadata, custom agent TOML, or thread/worktree
  readiness without parser evidence and the plugin config validator when it is
  available.
- Do not let the parent thread silently patch child-owned plugin architecture
  feedback. Route review feedback to the owning child thread and require that
  thread's verification evidence before marketplace readiness.
- Do not add root-level plugin manifests when the canonical layout is
  `plugins/<plugin>/.codex-plugin/plugin.json`.
- Do not present plugin-internal `.codex/agents/*.toml` files as canonical or
  supported; use `plugins/<plugin>/agents/catalog.toml` and
  `plugins/<plugin>/agents/roles/*.toml` for role metadata.
- Do not create package manager files solely to validate a static plugin unless
  the release or automation scope requires them.

## Evidence Rules

- JSON manifests require parser validation.
- LSP and MCP config require parser validation and, when the plugin being
  prepared is Codexy, `python3 scripts/validate-plugin-config.py --check`.
- Custom agent TOMLs and role metadata require parser validation and evidence
  that no separate orchestrator agent competes with the invoking thread.
- Skill bundles require frontmatter and metadata validation.
- Asset references require file-existence checks from the plugin root.
- Marketplace installability requires checking both the repo-root-relative
  marketplace entry and plugin manifest paths.

## Failure Modes

- Confusing a GitHub repository with an installable Codex plugin.
- Letting marketplace copy drift from implemented skills.
- Reintroducing obsolete root manifests or stale package scripts.
- Shipping plugin metadata that depends on a local absolute path.
