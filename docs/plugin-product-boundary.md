# Three-plugin product boundary

This is the public product boundary for the approved three-plugin line. It
freezes target ownership and records completed scoped extractions. It does not
publish or operate extension packages. The machine-readable contract is composed
of the metadata in
[`plugin-product-boundary.json`](plugin-product-boundary.json) and four
responsibility sidecars:
[`plugin-product-boundary-core.json`](plugin-product-boundary-core.json),
[`plugin-product-boundary-github.json`](plugin-product-boundary-github.json),
[`plugin-product-boundary-devtools.json`](plugin-product-boundary-devtools.json),
and
[`plugin-product-boundary-repository.json`](plugin-product-boundary-repository.json).
The contract test reads those exact files into one logical inventory, which is
the executable and sole source for current-path ownership. Its `surfaceRecords`
carry a stable logical-surface ID, concrete source path or registration, target,
and disposition. There is no parallel category-wide ownership map: the contract
test discovers every governed current surface and requires exact,
non-overlapping coverage by these records.

## Public products and packaging

| Product           | Public name     | Package root              | Responsibility                                                                                                                 |
| ----------------- | --------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `codexy`          | Codexy          | `plugins/codexy`          | Core orchestration, evidence, shared specialist and skill contracts, instruction enforcement, engineering, dreaming, and Wiki. |
| `codexy-github`   | Codexy GitHub   | `plugins/codexy-github`   | Optional GitHub workflow context, narrow title checks, and captured-state diagnostics using published core contracts.          |
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

### Optional GitHub behavior

Installing `codexy-github` does not grant GitHub access or consent to a
repository process. Ordinary issue, pull-request, review, and merge operations
continue through the host, connector, GitHub credentials, and branch
protections. The distributed component does not block, rewrite, or require a
body template, review quota, fixed approval phrase, or exclusive command route
for a general authorized mutation.

Three narrow checks remain effective on their supported hook paths: issue-title,
PR-title, and squash-subject validation. They check only the captured title or
merge subject and do not become an operation allowlist.

Retained hook checks:

| Hook check        | Question it answers                                                                  | Unsupported-source limit                                                                           |
| ----------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| Issue or PR title | Does the captured title satisfy the retained narrow title contract?                  | A title alone says nothing about authorization, body content, review, labels, or merge permission. |
| Squash subject    | Does the captured merge subject satisfy the retained title-derived subject contract? | It does not authorize, perform, or prove a merge.                                                  |

Optional captured-state diagnostics:

The optional `codexy-github-check` command can evaluate captured title,
PR-state, and merge-message data; running it is not a prerequisite for ordinary
GitHub work.

| Diagnostic                 | Question it answers                                                                                                | Deliberate use                                                                  | Unsupported-source limit                                                                                                    |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Captured issue or PR title | Does the captured title satisfy the retained narrow title contract?                                                | As a local preflight or when a repository-selected procedure asks for evidence. | A title alone says nothing about authorization, body content, review, labels, or merge permission.                          |
| Captured PR labels         | Does captured open-PR state contain repository label taxonomy and label application evidence?                      | When a repository-selected readiness procedure asks for that evidence.          | Missing or unsupported PR-state fields remain unavailable; the check does not decide whether a label policy is appropriate. |
| Captured merge message     | Does the captured subject use the validated PR title and expected PR suffix, with the selected issue closing line? | When a repository-selected merge-message procedure asks for that evidence.      | It does not authorize, perform, or prove a merge.                                                                           |

PR bodies remain free-form unless the repository owner selects a template. A
useful optional recommendation is `Summary`, `Rationale`, `Changed Areas`,
`Verification`, `Evidence`, `Not Run`, and `Follow-ups`; add a final
`Fixes
#<issue>` line only when the selected repository contract needs that
linkage. If a source lacks the fields needed for a selected diagnostic, report
it as unsupported or unknown rather than fabricating `PASS` or adding a new
global permission barrier.

## Target destinations and dispositions

| Current logical surface                                                                                      | Destination                  | Disposition                          |
| ------------------------------------------------------------------------------------------------------------ | ---------------------------- | ------------------------------------ |
| Orchestration, specialists, instruction hooks, dreaming, engineering, realtime voice orchestration, and Wiki | `codexy`                     | Retain in core.                      |
| Generic GitHub issue, branch, worktree, pull request, review, CI, merge, and release workflow                | `codexy-github`              | Extracted.                           |
| Codegraph and LSP MCP registrations, runtimes, wrappers, guidance, and permissions                           | `codexy-devtools`            | Extracted into the devtools package. |
| `release-engineering` and `plugin-marketplace-prep` skills                                                   | Repository-only Codex skills | Move out of the installed plugin.    |

Each row is one destination decision for its current logical surface. A later
implementation issue MAY refine a root into individual files only within its
assigned destination; it MUST NOT change the destination without updating this
contract and its verification.

## Current inventory mapping

Every mapped directory includes every current regular file below that root;
every listed file has exactly one target destination or disposition in the
preceding table. These are current source paths, including extracted GitHub
paths; they do not imply that every future extension already has files.

| Surface                                   | Current paths                                                                                                                                                                                                                                                                                       | Destination                                                                                                                                                                                                                                                                                                                                                             |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hooks                                     | `plugins/codexy/hooks/**`, `plugins/codexy-github/hooks/**`                                                                                                                                                                                                                                         | Core retains thread-delivery, child-thread-creation admission, subagent-ownership admission, and their non-GitHub policy envelope; the GitHub component owns workflow context and independent shell credential/filesystem/Git safety. GitHub mutations use host, connector, and GitHub authorization. Repository-specific `.codex` hooks remain at the repository root. |
| Skills                                    | `plugins/codexy/skills/**`, `plugins/codexy-github/skills/**`                                                                                                                                                                                                                                       | Core keeps orchestration/dreaming/engineering/realtime voice orchestration/Wiki; GitHub workflow is in `codexy-github`; release/marketplace remain repository-only.                                                                                                                                                                                                     |
| Agents                                    | `plugins/codexy/agents/**`, `plugins/codexy-github/agents/**`                                                                                                                                                                                                                                       | Core specialists remain `codexy`; `codexy-weaver`, which requires `git-workflow`, is in `codexy-github`.                                                                                                                                                                                                                                                                |
| MCP and runtime                           | `plugins/codexy-devtools/.mcp.json`, `plugins/codexy-devtools/mcp/**`, `packages/codexy-runtime/src/codegraph/**`, `packages/codexy-runtime/src/lsp/**`, `packages/codexy-runtime/src/mcp.rs`, `packages/codexy-runtime/src/bin/**`, `packages/codexy-runtime/src/version/**`                       | Codegraph/LSP and their wrappers/runtime entrypoints are owned by `codexy-devtools`; all other current runtime binaries and version modules remain repository-owned for their downstream module-owned packaging decision.                                                                                                                                               |
| LSP                                       | `plugins/codexy-devtools/.codex/lsp-client.json`, `plugins/codexy-devtools/lsp/**`, `packages/codexy-runtime/src/lsp/**`                                                                                                                                                                            | `codexy-devtools`                                                                                                                                                                                                                                                                                                                                                       |
| Assets                                    | `assets/**`, `plugins/codexy/assets/**`                                                                                                                                                                                                                                                             | Repository assets remain repository-only; plugin-local assets remain `codexy`.                                                                                                                                                                                                                                                                                          |
| Validators and tests                      | `scripts/sync-plugin-version.sh`, `scripts/validate-plugin-config.sh`, `packages/codexy-runtime/src/validation/**`, `packages/codexy-runtime/tests/**`                                                                                                                                              | Repository-owned validation; later product validators follow the target boundary.                                                                                                                                                                                                                                                                                       |
| Public entrypoints and packaging metadata | `README.md`, `README.ko.md`, `packages/getcodexy/{pyproject.toml,src/**,tests/**}`, `plugins/codexy/{bootstrap-codexy-agents,check-codexy-agents,.codex-plugin/plugin.json}`, `plugins/codexy/agents/openai.yaml`, `.agents/plugins/{marketplace,release-publish-contract,runtime-activation}.json` | Core identity and repository distribution surface; no layout migration here.                                                                                                                                                                                                                                                                                            |

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
