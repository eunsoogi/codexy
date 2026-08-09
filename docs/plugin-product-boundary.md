# Three-plugin product boundary

This is the public product boundary for the approved three-plugin line. It
freezes target ownership before extraction; it does not create, publish, or
operate the two reserved extension packages. The machine-readable inventory in
[`plugin-product-boundary.json`](plugin-product-boundary.json) is the
executable and sole source for current-path ownership. Its `surfaceRecords`
carry a stable logical-surface ID, concrete source path or registration,
target, and disposition. There is no parallel category-wide ownership map:
the contract test discovers every governed current surface and requires exact,
non-overlapping coverage by these records.

## Public products and packaging

| Product | Public name | Package root | Responsibility |
| --- | --- | --- | --- |
| `codexy` | Codexy | `plugins/codexy` | Core orchestration, evidence, shared specialist and skill contracts, instruction enforcement, engineering, dreaming, and Wiki. |
| `codexy-github` | Codexy GitHub | `plugins/codexy-github` (reserved) | GitHub issues, pull requests, reviews, and repository integration using published core contracts. |
| `codexy-devtools` | Codexy Devtools | `plugins/codexy-devtools` (reserved) | Local developer-tool, editor, CLI, and diagnostic integration using published core contracts. |

`codexy` remains the approved core identity: its manifest name, current package
root, public documentation, and installation identity MUST NOT be renamed as
part of this boundary freeze. The extension names and roots are reserved
packaging targets only; their manifests, releases, operations, and extraction
are outside this issue.

Codexy is a monorepo. The repository root is not a product package root. The
Rust runtime owns its build metadata at
`packages/codexy-runtime/{Cargo.toml,Cargo.lock,rust-toolchain.toml,rustfmt.toml,
clippy.toml,src,tests}` beside the Python distribution at
`packages/getcodexy/{pyproject.toml,src,tests}`, while plugins remain under
`plugins/`. A root Cargo workspace is not required and MUST NOT be assumed by
this contract. Root developer commands use repository scripts or an explicit
runtime manifest path/working directory.

## Public dependencies

| Consumer | May depend on | Forbidden dependencies |
| --- | --- | --- |
| `codexy` | None | `codexy-github`, `codexy-devtools` |
| `codexy-github` | Published `codexy` contract only | `codexy-devtools`; private core files or extension implementation details |
| `codexy-devtools` | Published `codexy` contract only | `codexy-github`; private core files or extension implementation details |

The extensions MUST NOT depend on one another. Core MUST NOT acquire a runtime,
build, packaging, import, skill-routing, agent-routing, MCP, LSP, hook, or
validator dependency on either extension. A future extraction MUST promote any
needed core capability into an explicitly documented public contract rather
than importing a private path across product roots.

## Target destinations and dispositions

| Current logical surface | Destination | Disposition |
| --- | --- | --- |
| Orchestration, specialists, instruction hooks, dreaming, engineering, and Wiki | `codexy` | Retain in core. |
| Generic GitHub issue, branch, worktree, pull request, review, CI, merge, and release workflow | `codexy-github` | Extract in the downstream GitHub-plugin issue. |
| Codegraph and LSP MCP registrations, runtimes, wrappers, guidance, and permissions | `codexy-devtools` | Extract in the downstream devtools issue. |
| `release-engineering` and `plugin-marketplace-prep` skills | Repository-only Codex skills | Move out of the installed plugin. |
| grep.app MCP registration | Remove | Remove in its downstream issue. |

Each row is one destination decision for its current logical surface. A later
implementation issue MAY refine a root into individual files only within its
assigned destination; it MUST NOT change the destination without updating this
contract and its verification.

## Current inventory mapping

Every mapped directory includes every current regular file below that root;
every listed file has exactly one target destination or disposition in the
preceding table. These are current source paths, not a claim that every source
path remains in core after its downstream extraction issue. The reserved
products own no physical files yet.

| Surface | Current paths | Destination |
| --- | --- | --- |
| Hooks | `plugins/codexy/hooks/**` | Core retains the complete current admission and instruction-enforcement import closure; generic GitHub workflow hooks extract. GitHub-policy leaves may move only after a later extraction breaks that core closure. |
| Skills | `plugins/codexy/skills/**` | Core keeps orchestration/dreaming/engineering/Wiki; GitHub workflow extracts; release/marketplace become repository-only. |
| Agents | `plugins/codexy/agents/**` | Core specialists remain `codexy`; `codexy-weaver`, which requires `git-workflow`, moves to `codexy-github`. |
| MCP and runtime | `plugins/codexy/.mcp.json`, `plugins/codexy/mcp/**`, `packages/codexy-runtime/src/codegraph/**`, `packages/codexy-runtime/src/lsp/**`, `packages/codexy-runtime/src/mcp.rs`, `packages/codexy-runtime/src/bin/**`, `packages/codexy-runtime/src/version/**` | Codegraph/LSP and their wrappers/runtime entrypoints move to `codexy-devtools`; all other current runtime binaries and version modules remain repository-owned for their downstream module-owned packaging decision; grep.app is removed. |
| LSP | `plugins/codexy/.codex/lsp-client.json`, `plugins/codexy/lsp/**`, `packages/codexy-runtime/src/lsp/**` | `codexy-devtools` |
| Assets | `assets/**`, `plugins/codexy/assets/**` | Repository assets remain repository-only; plugin-local assets remain `codexy`. |
| Validators and tests | `scripts/sync-plugin-version`, `scripts/validate-plugin-config`, `packages/codexy-runtime/src/validation/**`, `packages/codexy-runtime/tests/**` | Repository-owned validation; later product validators follow the target boundary. |
| Public entrypoints and packaging metadata | `README.md`, `README.ko.md`, `packages/getcodexy/{pyproject.toml,src/**,tests/**}`, `plugins/codexy/{bootstrap-codexy-agents,check-codexy-agents,.codex-plugin/plugin.json}`, `plugins/codexy/agents/openai.yaml`, `.agents/plugins/{marketplace,release-publish-contract,runtime-activation}.json` | Core identity and repository distribution surface; no layout migration here. |

The inventory test loads the JSON contract as its only ownership authority. It
discovers the governed current hook, skill, agent, MCP/runtime, LSP, asset,
validator/test, workflow, packaging, and public-entrypoint surfaces; then it
requires exact cover with non-empty stable-ID records. A typed, deny-unknown
schema makes `surfaceRecords` the only top-level ownership authority, while an
exhaustive stable-ID matrix fixes every record's target and disposition. The
product matrix fixes each public name, package root, and allowed/forbidden
dependency edge. The same validator reads concrete Python relative, package,
dotted, and wrapper imports, and typed agent metadata for required skills,
rejecting missing modules and forbidden core-to-extension edges. It
rejects omission,
overlap or duplicate source, stale MCP selector, unknown target or disposition,
illegal dependency, a parallel ownership projection, and an all-core
reassignment of the devtools registrations.
It is intentionally a freeze-time guard: it does not authorize physical
movement of any mapped path.

## Forbidden work in this boundary freeze

Physical extraction, getcodexy component operations, release-train changes,
new extension manifests, marketplace listings, and unrelated cleanup are out
of scope. Any such change needs a separately scoped issue and an updated
inventory contract.
