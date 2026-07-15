---
name: release-engineering
description: MUST use when preparing plugin versions, version sync, changelogs, release notes, tags, packaging, GitHub Actions release flows, distribution checks, rollback plans, or publish readiness.
---

# Release Engineering

## Purpose

Turn a verified change set into a reproducible release candidate. MUST keep
versions, manifests, release notes, artifacts, automation, and rollback evidence
aligned before publishing or tagging.

## Workflow

1. MUST identify release unit:
   - plugin manifest,
   - marketplace entry,
   - skill bundle,
   - MCP configuration,
   - LSP configuration and catalog,
   - codegraph MCP registration and code-exploration guidance,
   - role metadata or custom agent TOMLs,
   - thread/worktree orchestration guidance,
   - GitHub Action,
   - documentation bundle,
   - tag or GitHub release.
2. MUST find version sources of truth:
   - `plugins/<plugin>/.codex-plugin/plugin.json`,
   - `.agents/plugins/marketplace.json` when it carries install availability,
   - package metadata only when a package manager is intentionally present,
   - release workflow inputs,
   - changelog or release notes.
3. MUST choose version policy:
   - patch for compatible fixes,
   - minor for new skills or capabilities,
   - major for breaking invocation, manifest, or compatibility changes.
4. Synchronize versions across every declared source of truth.
5. Prepare release PR gates:
   - clean worktree,
   - manifest parser checks,
   - marketplace parser checks,
   - LSP config, MCP config, role metadata, custom agent TOML, and
     thread/worktree wording checks for plugin architecture changes, limited to
     the surfaces that exist for that plugin,
   - codegraph MCP registration checks when the release advertises code
     exploration or thread/agent repository discovery,
   - for Codexy plugin releases specifically,
     `scripts/validate-plugin-config --check` when the validator
     exists,
   - child-owned PR review feedback routed to the owning child thread with
     fresh verification before the parent thread merges,
   - skill metadata checks,
   - asset existence checks,
   - workflow syntax checks when GitHub Actions changed,
   - release notes or changelog when a user-facing version changes.
6. Validate artifact or package shape from the release unit, not only source
   files.
7. MUST inspect artifacts for secrets, local paths, debug files, oversized files,
   and unintended dependencies.
8. Publish, tag, or create GitHub releases only when explicitly requested by
   the active workflow.

## Required Output

```text
Release unit:
Current version:
Target version:
Version policy:
Files to sync:
Release PR gates:
Architecture validation:
Validation commands:
Artifact checks:
Rollback plan:
Not publishing because:
```

## Gates

- MUST NOT bump one version source without syncing the rest.
- MUST NOT publish from a dirty tree.
- MUST NOT tag before the release PR is merged unless the workflow explicitly
  requires pre-merge tags.
- MUST NOT treat source-tree validation as artifact validation when a package,
  archive, or marketplace bundle is produced.
- MUST NOT release plugin architecture changes while LSP, MCP, role metadata,
  custom agent TOML, or thread/worktree orchestration checks are missing.
- MUST NOT release code-exploration behavior without packaged codegraph MCP
  registration evidence.
- MUST NOT merge child-owned release or architecture feedback from the parent
  thread alone. The owning child thread MUST address or explicitly reject the
  feedback and return current verification.

## Evidence Rules

- Version sync MUST include direct file inspection or parser output.
- Architecture validation MUST include parser output for structured config and
  surface-specific checks for only the plugin surfaces that exist. For Codexy
  plugin releases, MUST run `scripts/validate-plugin-config --check`
  when present.
- GitHub Actions changes MUST require syntax or command-level validation where
  possible.
- Release notes MUST match the actual diff and merged PRs.
- Rollback plans MUST name the prior version, prior tag or commit, and how to
  restore installability.

## Failure Modes

- Bumping versions without updating release notes.
- Publishing from an unreviewed PR head.
- Adding release automation without a manual run and audit path.
- Mixing release prep with unrelated feature work.
