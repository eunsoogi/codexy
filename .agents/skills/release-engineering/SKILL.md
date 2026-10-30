---
name: release-engineering
description: MUST use when preparing plugin versions, version sync, changelogs, release notes, tags, packaging, GitHub Actions release flows, distribution checks, rollback plans, or publish readiness.
---

# Release Engineering

## Purpose

Turn a verified change set into a reproducible release candidate. MUST keep
release-owned version fields, release notes, workflows, artifacts, and rollback
evidence aligned before publishing or tagging.

## Workflow

1. MUST identify the release unit from the surfaces that actually exist:
   - plugin manifest;
   - marketplace entry;
   - skill bundle;
   - MCP or LSP configuration;
   - role metadata or custom agent TOMLs;
   - Codegraph MCP registration and code-exploration guidance;
   - thread/worktree orchestration guidance;
   - GitHub Action, documentation bundle, tag, or GitHub release.
2. MUST find every version source of truth: the plugin manifest at
   `plugins/<plugin>/.codex-plugin/plugin.json`,
   `.agents/plugins/marketplace.json` when it carries install availability,
   package metadata only when a package manager is intentionally present,
   release workflow inputs, and the changelog or release notes.
3. MUST choose the compatible version policy (patch for compatible fixes, minor
   for new skills or capabilities, major for breaking invocation, manifest, or
   compatibility changes) and synchronize the release-owned version fields.
   Direct file inspection or parser output MUST prove synchronization.
4. MUST complete the [Marketplace Release Handoff](#marketplace-release-handoff)
   for plugin or marketplace releases. It supplies the current source-readiness
   result and the plugin-configuration checks owned by Marketplace Prep.
5. When GitHub Actions change, MUST validate workflow syntax or command behavior
   where possible. Workflow paths MUST resolve from the repository root, and
   release workflow version inputs MUST agree with the release-owned version
   sources.
6. MUST validate artifact shape from the actual release unit and inspect the
   resulting artifact for secrets, local paths, debug files, oversized files,
   and unintended dependencies. Source-tree checks alone MUST NOT stand in for
   artifact inspection.
7. When a user-facing version changes, MUST prepare release notes or a changelog
   and keep them consistent with the actual change and merged work. A rollback
   plan MUST identify the prior version, tag or commit, and steps to restore
   installability.
8. MUST publish, tag, or create a GitHub release only when the active workflow
   explicitly authorizes that action.

## Marketplace Release Handoff

This is the single procedure connecting Marketplace Prep and Release
Engineering. Marketplace Prep owns discovery, structure, non-version metadata,
and asset references. Release Engineering owns version values, artifacts,
publication, and rollback.

1. Marketplace Prep MUST validate the plugin and marketplace source for the
   candidate revision. Its result MUST identify that revision, the plugin and
   marketplace paths, applicable validation results, and unresolved gaps. It
   proves source readiness only.
2. Release Engineering MUST consume that result for the same candidate revision
   and use it for marketplace-owned structure, metadata, asset, and plugin
   configuration checks. Plugin-architecture release readiness MUST include all
   applicable supported surfaces from that result. Release work MUST NOT copy
   those checklists. If release work changes a marketplace-owned field, path, or
   asset, Marketplace Prep MUST refresh the affected checks for the new revision
   before release readiness is claimed.
3. When Marketplace Prep evaluates a listing or install candidate that names a
   release version, tag, artifact, or published state, it MUST consume Release
   Engineering's result for the same candidate and verify any external
   availability claim from the corresponding release surface. Without that
   result, it MUST limit its conclusion to source readiness and identify the
   missing release evidence.
4. Source validation, artifact inspection, actual installation, and external
   publication each prove a separate claim. Neither skill may substitute one
   surface's evidence for another. Evidence reuse and completion claims MUST
   follow `$proof-driven-completion`.
5. GitHub issue, PR, review, and handoff work MUST follow `$orchestration` and
   `$git-workflow`.

## Required Output

```text
Candidate revision:
Release unit:
Current version:
Target version:
Version policy:
Files to sync:
Release PR gates:
Marketplace readiness:
Architecture validation:
Validation commands:
Artifact checks:
Publication state:
Rollback plan:
Not publishing because:
```

## Gates

- MUST NOT change a release-owned version field without synchronizing its other
  sources of truth.
- MUST NOT claim a release candidate is ready from a dirty worktree.
- MUST NOT publish from a dirty tree or tag before merge unless the workflow
  explicitly requires a pre-merge tag.
- MUST NOT claim artifact, installation, or publication readiness from
  source-tree validation.
- MUST NOT claim a plugin release is marketplace-ready without the current
  Marketplace Release Handoff result.

## Failure Modes

- Updating one version field while leaving another source stale.
- Publishing an artifact whose contents or provenance were not inspected.
- Treating a source manifest or marketplace entry as proof of installation or
  publication.
- Omitting release notes or a changelog for a user-facing version change.
- Adding release automation without a manual run and audit path.
- Mixing release preparation with unrelated feature work.
