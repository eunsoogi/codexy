# getcodexy component installation contract

This is the target public contract for the 1.4.0 component-installation CLI.
It is not implemented by the current 1.3.0 `getcodexy` distribution. The
normative machine-readable source is
`packages/getcodexy/contracts/component-installation-contract.json`; executable
examples live in `packages/getcodexy/tests/fixtures/component-installation-cases.json`.

## Components and source of truth

The logical component names, in canonical output order, are `core`, `github`,
and `devtools`. They correspond to the public plugin identities `codexy`,
`codexy-github`, and `codexy-devtools` respectively. `github` and `devtools`
each depend on `core`.

The successful installed component inventory is the source of truth. A command
request expresses intent and a receipt records the result, but neither replaces
the inventory. A present installed inventory that does not satisfy the dependency
graph is inconsistent and commands report `inconsistent-installed-state` before a
mutation. An absent inventory is distinct: `update` reports
`no-recorded-selection` because there is no selection to preserve.

This contract deliberately names only logical identities. It does not prescribe
filesystem paths, package roots, manifests, resolver data, transaction storage,
or release layout.

## Commands

| Command | Selection rule | Mutation rule |
| --- | --- | --- |
| `getcodexy install [COMPONENT ...]` | No components selects all. Explicit components include their transitive dependencies. | Adds the resolved selection to the installed selection; it never removes another installed component. |
| `getcodexy update [COMPONENT ...]` | With no components, updates all installed components. With components, updates their resolved subset of the installed selection. | Preserves the installed selection. |
| `getcodexy remove COMPONENT [COMPONENT ...]` | Requires at least one component. | Rejects a request if a retained component would still depend on a requested removal. |
| `getcodexy status` | Reads the installed inventory. | None. |
| `getcodexy doctor` | Reads inventory consistency, host readiness, and component health. | None. |
| `getcodexy bootstrap` | Selects the complete supported installation. | Installs all components and performs the required host activation. |

Unknown components fail with `unknown-component`. `update` without any recorded
inventory fails with `no-recorded-selection`; a present but dependency-invalid
inventory fails with `inconsistent-installed-state`. A command that does not
accept component operands returns `components-not-accepted`.

## State transitions

| Before | Command | After | Outcome |
| --- | --- | --- | --- |
| none | `install` | core, github, devtools | completed |
| none | `install core` | core | completed |
| none | `install github` | core, github | completed |
| none | `install devtools` | core, devtools | completed |
| core, devtools | `install github` | core, github, devtools | completed |
| core, devtools | `update` | core, devtools | completed |
| core, github, devtools | `remove github` | core, devtools | completed |
| core, github | `remove core` | core, github | rejected: dependency-protected-removal |
| core, github | `remove core github` | none | completed |
| any consistent selection | a mutating operation fails | exact pre-operation selection | rolled-back |

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

All public commands accept `--json`. In JSON mode, stdout contains exactly one JSON
object and no human-oriented output. Mutation receipts use
`getcodexy.operation-receipt.v1`; `status` uses the distinct
`getcodexy.status.v1` schema. Stable `error.code` values are authoritative;
specific numeric exit-code assignments remain an implementation decision.
