---
name: release-engineering
description: Use when preparing plugin versions, version sync, changelogs, release notes, tags, packaging, GitHub Actions release flows, distribution checks, rollback plans, or publish readiness.
---

# Release Engineering

## Purpose

Turn a verified change set into a reproducible release candidate. Keep
versions, manifests, release notes, artifacts, automation, and rollback evidence
aligned before publishing or tagging.

## Workflow

1. Identify release unit:
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
2. Find version sources of truth:
   - `plugins/<plugin>/.codex-plugin/plugin.json`,
   - `.agents/plugins/marketplace.json` when it carries install availability,
   - package metadata only when a package manager is intentionally present,
   - release workflow inputs,
   - changelog or release notes.
3. Choose version policy:
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
     `python3 scripts/validate-plugin-config.py --check` when the validator
     exists,
   - child-owned PR review feedback routed to the owning child thread with
     fresh verification before the parent thread merges,
   - skill metadata checks,
   - asset existence checks,
   - workflow syntax checks when GitHub Actions changed,
   - Codex review on latest PR head,
   - release notes or changelog when a user-facing version changes.
6. Validate artifact or package shape from the release unit, not only source
   files.
7. Inspect artifacts for secrets, local paths, debug files, oversized files,
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

- Do not bump one version source without syncing the rest.
- Do not publish from a dirty tree.
- Do not tag before the release PR is merged unless the workflow explicitly
  requires pre-merge tags.
- Do not treat source-tree validation as artifact validation when a package,
  archive, or marketplace bundle is produced.
- Do not release plugin architecture changes while LSP, MCP, role metadata,
  custom agent TOML, or thread/worktree orchestration checks are missing.
- Do not release code-exploration behavior without packaged codegraph MCP
  registration evidence.
- Do not merge child-owned release or architecture feedback from the parent
  thread alone. The owning child thread must address or explicitly reject the
  feedback and return current verification.

## Evidence Rules

- Version sync requires direct file inspection or parser output.
- Architecture validation requires parser output for structured config and
  surface-specific checks for only the plugin surfaces that exist. For Codexy
  plugin releases, run `python3 scripts/validate-plugin-config.py --check`
  when present.
- GitHub Actions changes require syntax or command-level validation where
  possible.
- Release notes must match the actual diff and merged PRs.
- Rollback plans must name the prior version, prior tag or commit, and how to
  restore installability.

## Failure Modes

- Bumping versions without updating release notes.
- Publishing from an unreviewed PR head.
- Adding release automation that cannot be run manually or audited.
- Mixing release prep with unrelated feature work.
