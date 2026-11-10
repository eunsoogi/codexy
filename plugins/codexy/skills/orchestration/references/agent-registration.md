# Agent Registration And Invocation

## Packaged Artifact Boundary

Codexy ships specialist custom-agent TOMLs at
`plugins/codexy/agents/<name>.toml`, with discovery metadata in
`plugins/codexy/agents/catalog.toml`; MUST keep one specialist per file.
`plugins/codexy/agents/openai.yaml` is the plugin invocation interface, not a
specialist worker. Installed specialists require the stable registration bridge
and independent schema/invocation preflight. MUST NOT treat
`plugins/codexy/.codex/agents` as installed custom agents.

## Source-Only Pre-Session Update

The installed package may include a pre-session updater implementation for
activation work. It MUST NOT be presented as a public command until the package
has published its console entry point. No-change execution remains a packaged
behavior covered by dedicated tests. The packaged `check-codexy-agents`
entrypoint remains an explicit read-only validator and MUST report
`UPDATE_REQUIRED` when installed projections differ from the current package;
lifecycle hooks MUST NOT invoke it.

When an exact packaged `agent_type` is unavailable, MUST resolve this selected
skill's installed directory and run its package-owned sibling
`scripts/bootstrap-codexy-agents` entrypoint. MUST NOT resolve the entrypoint
from the active project, an unrelated source checkout, or a hard-coded cache
path. The bootstrap diagnoses the installed state before mutation and invokes
`register_codexy_agents.py` only when packaged role discovery is incomplete.

If the bootstrap reports `D bootstrap: RESTART_REQUIRED`, MUST stop specialist
dispatch in the current task and MUST tell the user to restart Codex or start a
fresh task. The stale task MUST NOT claim that newly projected roles are
callable. In the fresh task, MUST observe `agent_type` and invoke the exact
packaged role before claiming success.

If the bootstrap reports `D bootstrap: READY` but the exact role is still
unavailable, registration is not the defect. MUST record the active tool-schema
or host-exposure mismatch and fail closed. MUST NOT substitute `default`,
`worker`, or `explorer` for a Codexy specialist or Sentinel.

The registration bridge and update checker MUST NOT run from SessionStart,
UserPromptSubmit, or another lifecycle hook. Codexy MUST NOT commit generated
MCP binaries to the package source; the published runtime bootstrap remains the
supported MCP installation path.

## Registration Lifecycle

The packaged bridge projects marker-owned TOMLs into
`$CODEX_HOME/agents/codexy/`, which Codex recursively discovers without
retaining versioned plugin-cache paths.

The bridge MUST migrate the legacy Codexy-managed `[agents.<name>]` block, MUST
NOT overwrite unmarked role files or unmanaged config declarations, and
`--uninstall` MUST remove only marker-owned files and the legacy managed block.
It trusts only the root-owned top-level filesystem boundary (canonicalizing a
platform alias such as macOS `/var`) and MUST reject symlink or reparse-point
components beneath that boundary. Ordinary lifecycle failures MUST roll back the
files and directories mutated by the attempt. This is not process-crash or
power-loss atomicity, and a hostile writer can still race the final portable
filesystem operation after its immediate revalidation.

Every new native `spawn_agent` request for a permitted role MUST explicitly set
`fork_turns="none"`. This applies equally to `explorer` and every registered
specialist; omitted, null, history-copying, bounded-positive, and other values
are denied before execution. A `send_message` or `followup_task` to an existing
permitted helper reuses that helper and is not a new spawn, so it does not need
a new `fork_turns` field or create a new-context gate. When correcting a
history-setting denial, MUST keep already-authorized task and model choices;
MUST NOT ask the user to restate them.

Each new child assignment MUST be bounded and self-contained. It MUST carry the
objective, latest relevant user corrections, owned scope, authority and
restrictions, relevant owner/head/artifact identities, completion criteria,
unresolved findings, and required verification. Include only the necessary facts
and precise source or artifact references, then retrieve further authorized
evidence selectively. MUST NOT paste the full parent transcript, unchanged full
tool bodies, or the full agent tree. Keep unavailable sources and missing
required evidence explicit rather than dropping them to meet a size target. The
explicit request value proves only what was requested; claim that a child
received no inherited turns only when the host exposes authoritative evidence
for that behavior. This changes what a new child receives; it does not delete or
rewrite parent history or alter source-retention and privacy policy.

Before claiming a specialist is callable, MUST run the registration script with
`--diagnose` and treat its rows independently:

- `role-discovery` proves the exact packaged standalone projections exist.
- `tool-schema` reports host settings only from the real
  `[features.multi_agent_v2]` table and MUST still require observing
  `agent_type` in a fresh task.
- An explicit custom `agent_type` MUST use the registered role, model, and
  reasoning contract. Its native spawn MUST also use the no-history value above.

Codexy MUST NOT manage `features.multi_agent_v2`. Upstream host compatibility
settings such as tool namespace and metadata visibility are diagnostic evidence
only. Fresh-task proof MUST name the host configuration used.
