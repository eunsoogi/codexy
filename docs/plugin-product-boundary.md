# Three-plugin product boundary

This is the public product boundary for the approved three-plugin line. It
freezes target ownership and records completed scoped extractions. It does not
publish or operate extension packages. The machine-readable inventory in
[`plugin-product-boundary.json`](plugin-product-boundary.json) is the executable
and sole source for current-path ownership. Its `surfaceRecords` carry a stable
logical-surface ID, concrete source path or registration, target, and
disposition. There is no parallel category-wide ownership map: the contract test
discovers every governed current surface and requires exact, non-overlapping
coverage by these records.

## Public products and packaging

| Product           | Public name     | Package root              | Responsibility                                                                                                                 |
| ----------------- | --------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `codexy`          | Codexy          | `plugins/codexy`          | Core orchestration, evidence, shared specialist and skill contracts, instruction enforcement, engineering, dreaming, and Wiki. |
| `codexy-github`   | Codexy GitHub   | `plugins/codexy-github`   | GitHub issues, pull requests, reviews, and repository integration using published core contracts.                              |
| `codexy-devtools` | Codexy Devtools | `plugins/codexy-devtools` | Local developer-tool, editor, CLI, and diagnostic integration using published core contracts.                                  |

`codexy` remains the approved core identity: its manifest name, current package
root, public documentation, and installation identity MUST NOT be renamed as
part of this boundary freeze. Extension installation operations are owned by the
component installer contract; physical package ownership is recorded here.

Codexy is a monorepo. The repository root is not a product package root. The
Rust runtime owns its build metadata at
`packages/codexy-runtime/{Cargo.toml,Cargo.lock,rust-toolchain.toml,rustfmt.toml, clippy.toml,src,tests}`
beside the Python distribution at
`packages/getcodexy/{pyproject.toml,src,tests}`, while plugins remain under
`plugins/`. A root Cargo workspace is not required and MUST NOT be assumed by
this contract. Root developer commands use repository scripts or an explicit
runtime manifest path/working directory.

## Public dependencies

| Consumer          | May depend on                    | Forbidden dependencies                                                    |
| ----------------- | -------------------------------- | ------------------------------------------------------------------------- |
| `codexy`          | None                             | `codexy-github`, `codexy-devtools`                                        |
| `codexy-github`   | Published `codexy` contract only | `codexy-devtools`; private core files or extension implementation details |
| `codexy-devtools` | Published `codexy` contract only | `codexy-github`; private core files or extension implementation details   |

The extensions MUST NOT depend on one another. Core MUST NOT acquire a runtime,
build, packaging, import, skill-routing, agent-routing, MCP, LSP, hook, or
validator dependency on either extension. A future extraction MUST promote any
needed core capability into an explicitly documented public contract rather than
importing a private path across product roots.

## Target destinations and dispositions

| Current logical surface                                                                       | Destination                  | Disposition                          |
| --------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------ |
| Orchestration, specialists, instruction hooks, dreaming, engineering, realtime voice orchestration, and Wiki | `codexy`                     | Retain in core.                      |
| Generic GitHub issue, branch, worktree, pull request, review, CI, merge, and release workflow | `codexy-github`              | Extracted.                           |
| Codegraph and LSP MCP registrations, runtimes, wrappers, guidance, and permissions            | `codexy-devtools`            | Extracted into the devtools package. |
| `release-engineering` and `plugin-marketplace-prep` skills                                    | Repository-only Codex skills | Move out of the installed plugin.    |

Each row is one destination decision for its current logical surface. A later
implementation issue MAY refine a root into individual files only within its
assigned destination; it MUST NOT change the destination without updating this
contract and its verification.

## Current inventory mapping

Every mapped directory includes every current regular file below that root;
every listed file has exactly one target destination or disposition in the
preceding table. These are current source paths, including extracted GitHub
paths; they do not imply that every future extension already has files.

| Surface                                   | Current paths                                                                                                                                                                                                                                                                                       | Destination                                                                                                                                                                                                                                                                                              |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hooks                                     | `plugins/codexy/hooks/**`, `plugins/codexy-github/hooks/**`                                                                                                                                                                                                                                         | Core retains thread-delivery, child-thread-creation admission, and their non-GitHub policy envelope; every generic GitHub, repository-GitHub, merge, review, and related policy-runtime closure is in `codexy-github`. Repository-specific `.codex` policy configuration remains at the repository root. |
| Skills                                    | `plugins/codexy/skills/**`, `plugins/codexy-github/skills/**`                                                                                                                                                                                                                                       | Core keeps orchestration/dreaming/engineering/realtime voice orchestration/Wiki; GitHub workflow is in `codexy-github`; release/marketplace remain repository-only.                                                                                                                                  |
| Agents                                    | `plugins/codexy/agents/**`, `plugins/codexy-github/agents/**`                                                                                                                                                                                                                                       | Core specialists remain `codexy`; `codexy-weaver`, which requires `git-workflow`, is in `codexy-github`.                                                                                                                                                                                                 |
| MCP and runtime                           | `plugins/codexy-devtools/.mcp.json`, `plugins/codexy-devtools/mcp/**`, `packages/codexy-runtime/src/codegraph/**`, `packages/codexy-runtime/src/lsp/**`, `packages/codexy-runtime/src/mcp.rs`, `packages/codexy-runtime/src/bin/**`, `packages/codexy-runtime/src/version/**`                       | Codegraph/LSP and their wrappers/runtime entrypoints are owned by `codexy-devtools`; all other current runtime binaries and version modules remain repository-owned for their downstream module-owned packaging decision.                                                                                |
| LSP                                       | `plugins/codexy-devtools/.codex/lsp-client.json`, `plugins/codexy-devtools/lsp/**`, `packages/codexy-runtime/src/lsp/**`                                                                                                                                                                            | `codexy-devtools`                                                                                                                                                                                                                                                                                        |
| Assets                                    | `assets/**`, `plugins/codexy/assets/**`                                                                                                                                                                                                                                                             | Repository assets remain repository-only; plugin-local assets remain `codexy`.                                                                                                                                                                                                                           |
| Validators and tests                      | `scripts/sync-plugin-version.sh`, `scripts/validate-plugin-config.sh`, `packages/codexy-runtime/src/validation/**`, `packages/codexy-runtime/tests/**`                                                                                                                                              | Repository-owned validation; later product validators follow the target boundary.                                                                                                                                                                                                                        |
| Public entrypoints and packaging metadata | `README.md`, `README.ko.md`, `packages/getcodexy/{pyproject.toml,src/**,tests/**}`, `plugins/codexy/{bootstrap-codexy-agents,check-codexy-agents,.codex-plugin/plugin.json}`, `plugins/codexy/agents/openai.yaml`, `.agents/plugins/{marketplace,release-publish-contract,runtime-activation}.json` | Core identity and repository distribution surface; no layout migration here.                                                                                                                                                                                                                             |

The inventory test loads the JSON contract as its only ownership authority. It
discovers the governed current hook, skill, agent, MCP/runtime, LSP, asset,
validator/test, workflow, packaging, and public-entrypoint surfaces; then it
requires exact cover with non-empty stable-ID records. A typed, deny-unknown
schema makes `surfaceRecords` the only top-level ownership authority, while an
exhaustive stable-ID matrix fixes every record's target and disposition. The
product matrix fixes each public name, package root, and allowed/forbidden
dependency edge. The same validator reads concrete Python relative, package,
dotted, and wrapper imports, and typed agent metadata for required skills,
rejecting missing modules and forbidden core-to-extension edges. It rejects
omission, overlap or duplicate source, stale MCP selector, unknown target or
disposition, illegal dependency, a parallel ownership projection, and an
all-core reassignment of the devtools registrations. It is an ownership guard: a
scoped extraction updates the inventory and its verification in the same change.

## Forbidden work in this boundary freeze

Getcodexy component operations, release-train changes, and unrelated cleanup
remain out of scope. Any such change needs a separately scoped issue and an
updated inventory contract.
