# Agent Registration And Invocation

## Installed First-Use Bootstrap

After marketplace installation or an official plugin update, users SHOULD run
the plugin-root `bootstrap-codexy-agents` before starting Codex. The README
one-line command resolves the enabled plugin from `codex plugin list --json`
without requiring users to know the marketplace snapshot or cache path. `--check`
MUST remain read-only and MUST report `UPDATE_REQUIRED` when installed role
projections differ from the current package.
SessionStart MUST invoke only the packaged `check-codexy-agents` entrypoint,
which has no registration mode and performs only read-only file comparisons.

When an exact Codexy `agent_type` is unavailable, MUST resolve this selected
skill's installed directory and run its sibling
`scripts/bootstrap-codexy-agents` entrypoint. MUST NOT resolve the entrypoint
from the target repository, a Codexy source checkout, or a hard-coded plugin
cache path. The bootstrap diagnoses the installed state before mutation and
invokes `register-codexy-agents` only when packaged role discovery is incomplete.

If the bootstrap reports `D bootstrap: RESTART_REQUIRED`, MUST stop specialist
dispatch in the current task and MUST tell the user to restart Codex or start a
fresh task. The stale task MUST NOT claim that newly projected roles are
callable. In the fresh task, MUST observe `agent_type` and invoke the exact
packaged role before claiming success.

If the bootstrap reports `D bootstrap: READY` but the exact role is still
unavailable, registration is not the defect. MUST record the active tool-schema
or host-exposure mismatch and fail closed. MUST NOT substitute `default`,
`worker`, or `explorer` for a Codexy specialist or Sentinel.

The registration bridge MUST NOT run from SessionStart, UserPromptSubmit, or
another lifecycle hook. SessionStart MAY rerun only the plugin-root `--check`
mode and MUST NOT mutate user state. Codexy MUST NOT commit generated MCP
binaries to the source plugin; the existing GitHub Release runtime bootstrap
remains the supported MCP installation path.

## Registration Lifecycle

The packaged bridge projects marker-owned TOMLs into
`$CODEX_HOME/agents/codexy/`, which Codex recursively discovers without retaining
versioned plugin-cache paths.

The bridge MUST migrate the legacy Codexy-managed `[agents.<name>]` block, MUST
NOT overwrite unmarked role files or unmanaged config declarations, and
`--uninstall` MUST remove only marker-owned files and the legacy managed block.
It trusts only the root-owned top-level filesystem boundary (canonicalizing a
platform alias such as macOS `/var`) and MUST reject symlink or reparse-point
components beneath that boundary. Ordinary lifecycle failures MUST roll back
the files and directories mutated by the attempt. This is not process-crash or
power-loss atomicity, and a hostile writer can still race the final portable
filesystem operation after its immediate revalidation.

Before claiming a specialist is callable, MUST run the registration script with
`--diagnose` and treat its rows independently:

- `role-discovery` proves the exact packaged standalone projections exist.
- `tool-schema` reports host settings only from the real
  `[features.multi_agent_v2]` table and MUST still require observing `agent_type`
  in a fresh task.
- An explicit custom `agent_type` MUST use `fork_turns="none"` or a positive bounded count.
  MultiAgent V2 full-history `fork_turns="all"` is incompatible
  with role, model, or reasoning overrides.

Codexy MUST NOT manage `features.multi_agent_v2`. Upstream host compatibility
settings such as tool namespace and metadata visibility are diagnostic evidence
only. Fresh-task proof MUST name the host configuration used.
