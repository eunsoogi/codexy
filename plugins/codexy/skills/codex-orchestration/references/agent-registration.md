# Agent Registration And Invocation

MUST run `skills/codex-orchestration/scripts/register-codexy-agents` from the
installed plugin to register or update specialists, then MUST restart Codex or
MUST start a fresh task. The bridge projects marker-owned TOMLs into
`$CODEX_HOME/agents/codexy/`, which Codex recursively discovers without retaining
versioned plugin-cache paths.

The bridge MUST migrate the legacy Codexy-managed `[agents.<name>]` block, MUST
NOT overwrite unmarked role files or unmanaged config declarations, and
`--uninstall` MUST remove only marker-owned files and the legacy managed block.

Before claiming a specialist is callable, MUST run the registration script with
`--diagnose` and treat its rows independently:

- `role-discovery` proves the stable files exist.
- `tool-schema` MUST still require observing `agent_type` in a fresh task.
- An explicit custom `agent_type` MUST use `fork_turns="none"` or a positive bounded count.
  MultiAgent V2 full-history `fork_turns="all"` is incompatible
  with role, model, or reasoning overrides.

Codexy MUST NOT manage `features.multi_agent_v2`. Upstream host compatibility
settings such as tool namespace and metadata visibility are diagnostic evidence
only. Fresh-task proof MUST name the host configuration used.
