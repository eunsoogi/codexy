# getcodexy component installation contract

This is the target public contract for the 1.4.0 component-installation CLI. Its
complete component lifecycle is not implemented by the current 1.3.0 `getcodexy`
distribution. The executable component source is the packaged
`codexy_runtime_tools/component-manifest.json`; the public contract references
that resource from
`packages/getcodexy/contracts/component-installation-contract.json`. Examples
live in `packages/getcodexy/tests/fixtures/component-installation-cases.json`.

`codexy-github-install` is an optional 1.3.0 getcodexy transaction helper. A
trusted host may call it with its absolute executable path to install and verify
`codexy` before `codexy-github` and to project the optional Weaver registration.
It MUST NOT be required for direct Codex plugin installation: the installed
GitHub plugin's manifest, skill, agent file, and host-resolved hooks are its
native activation surface.

`codexy-github-check` is an optional public package command for generic issue
titles, PR titles, PR label evidence, and merge-message checks. Its inputs are
captured title, PR-state, and merge-message data; it MUST NOT be given a plugin
cache or repository-relative executable path.

## Components and source of truth

The logical component names, in canonical output order, are `core`, `github`,
and `devtools`. The packaged component manifest owns their public plugin
identities, lockstep version, plugin roots, plugin-local assets, dependencies,
compatible selections, and a packaged projection of the closed public
`domain_errors` set. The installation contract is authoritative for those stable
error codes; the Python loader and public Rust validator reject a manifest
projection that differs from it. `github` and `devtools` each depend on `core`.

The successful installed component inventory is the source of truth. A command
request expresses intent and a receipt records the result, but neither replaces
the inventory. A present installed inventory that does not satisfy the
dependency graph is inconsistent and commands report
`inconsistent-installed-state` before a mutation. An absent inventory is
distinct: `update` reports `no-recorded-selection` because there is no selection
to preserve.

The resolver validates requests and the manifest before any installer mutation.
After a future operation, it reconciles only the fresh
`codex plugin list --json` inventory; a requested selection or a pre-operation
receipt is never substituted for that installed-state source of truth.
Transaction storage and mutation execution remain owned by Issue #557.

The resolver accepts a coherent earlier lockstep installed version for update
planning, but a successful post-operation reconciliation requires the manifest's
exact lockstep version. Mixed versions, duplicate component records, an unknown
official Codexy component, or a known component from a different marketplace
fail before any mutation is planned.

Inventory reconciliation accepts only the official `codexy` marketplace root
resolved by the host. Each component record must name that marketplace, its
canonical plugin identity, and the matching absolute plugin path below that
root. A stale or malformed record cannot be treated as an unrelated plugin.

## Commands

| Command                                      | Selection rule                                                                                                                   | Mutation rule                                                                                                  |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `getcodexy install [COMPONENT ...]`          | No components selects all. Explicit components include their transitive dependencies.                                            | Adds the resolved selection to the installed selection; it never removes another installed component.          |
| `getcodexy update [COMPONENT ...]`           | With no components, updates all installed components. With components, updates their resolved subset of the installed selection. | Preserves the installed selection.                                                                             |
| `getcodexy remove COMPONENT [COMPONENT ...]` | Requires at least one component.                                                                                                 | Rejects a request if a retained component would still depend on a requested removal.                           |
| `getcodexy status`                           | Reads the installed inventory.                                                                                                   | None.                                                                                                          |
| `getcodexy doctor`                           | Reads inventory consistency, host readiness, and component health.                                                               | None.                                                                                                          |
| `getcodexy bootstrap`                        | Selects the complete supported installation.                                                                                     | Delegates to the transactional default install, whose enabled-plugin readback is the required host activation. |

Unknown components fail with `unknown-component`. `update` without any recorded
inventory fails with `no-recorded-selection`; a present but dependency-invalid
inventory fails with `inconsistent-installed-state`. A command that does not
accept component operands returns `components-not-accepted`. Resolver inventory
validation additionally reports `unknown-installed-component`,
`conflicting-installed-state`, `mixed-version-state`, or
`component-version-mismatch` before an operation is allowed to mutate plugins.

## State transitions

| Before                   | Command                    | After                         | Outcome                                |
| ------------------------ | -------------------------- | ----------------------------- | -------------------------------------- |
| none                     | `install`                  | core, github, devtools        | completed                              |
| none                     | `install core`             | core                          | completed                              |
| none                     | `install github`           | core, github                  | completed                              |
| none                     | `install devtools`         | core, devtools                | completed                              |
| core, devtools           | `install github`           | core, github, devtools        | completed                              |
| core, devtools           | `update`                   | core, devtools                | completed                              |
| core, github, devtools   | `remove github`            | core, devtools                | completed                              |
| core, github             | `remove core`              | core, github                  | rejected: dependency-protected-removal |
| core, github             | `remove core github`       | none                          | completed                              |
| any consistent selection | a mutating operation fails | exact pre-operation selection | rolled-back                            |

## Rollback

Every failed mutating operation automatically restores the exact pre-operation
installed selection and returns terminal `outcome: "rolled-back"`. Its JSON
operation receipt includes a stable `operation_id`, the attempted command,
requested and resolved components, selection before and after, installed
components, source of truth, and structured error codes.

There is deliberately no `getcodexy rollback RECEIPT_ID` command in this
contract. Manual rollback syntax, durable receipt storage, retention, lookup,
replay, and recovery authorization belong to Issue #557's transaction engine.

## Machine-readable output

All public commands accept `--json`. In JSON mode, stdout contains exactly one
JSON object and no human-oriented output. Mutation receipts use
`getcodexy.operation-receipt.v1`; `status` uses the distinct
`getcodexy.status.v1` schema; and `doctor` uses `getcodexy.doctor.v1`.

Status includes `inventory` and `inventory_consistency`. An absent inventory is
`{"state":"absent"}` with `not-recorded`; a present empty inventory is
`{"state":"present","components":[]}` with `consistent`; and a present
dependency-invalid inventory is `inconsistent` with an
`inconsistent-installed-state` error. This lets a client predict whether bare
`update --json` will reject for no recorded selection, complete without selected
components, or reject for an inconsistent installed state. Doctor additionally
includes `host_readiness` and canonical `component_health` entries, alongside
the same inventory consistency report.

Both read commands take a fresh `codex plugin list --json` snapshot and never
acquire a lifecycle lock, recover a journal, write a receipt, execute an MCP
wrapper, or invoke an activation helper. `selected_components` is the durable
selection record when present; `installed_components` is the fresh host
snapshot. Doctor reports only present or selected components, classifying them
as `healthy`, `missing`, `stale`, or `incompatible`, and attaches a declarative
repair. Recoverable missing and stale states use `getcodexy bootstrap`;
incompatible registrations require repair before the next doctor run.

`bootstrap --json` emits its own typed transactional receipt with
`command: "bootstrap"`. It is idempotent and reaches the same full default
selection as `install --json`, while retaining its own durable operation
identity, host readback, and automatic rollback semantics.

Stable `error.code` values are authoritative; specific numeric exit-code
assignments remain an implementation decision.
