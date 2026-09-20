# Support and CI contract

The repository-only `mcp-test` surface is intentionally narrower than a general
MCP client. `support` reports the same contract used by the producers:

- protocol: `2024-11-05`;
- transport: newline-delimited local stdio (`stdio-newline-v1`);
- platform: POSIX only;
- execution: trusted local subprocesses with explicit `argv`, `cwd`, tool
  allowlists, selected fields, bounded output, and finite deadlines.

Unsupported protocol versions, transports, and platforms fail closed. Windows
must be rejected before a server process is launched until native descendant
ownership is proven. This surface does not claim remote authentication,
streamable HTTP, ambient environment inheritance, or host/session skill-call
success.

## Exit codes

`run` uses `0` only for a successful scenario whose every step meets its
declared expectation. A failed step, predecessor suppression, deadline,
malformed response, launch error, unsupported response, or invalid manifest uses
`2`.

`compare` uses the comparison producer's CI contract:

- `0`: baseline and candidate both succeeded and matched;
- `1`: both completed but a selected behavior, value, error, shape, or linkage
  differed;
- `2`: either run failed or the result is incomparable.

No exit code is inferred from a single `tools/call`. A multi-step scenario must
finish its ordered chain, and a dependent step is suppressed after a failed
predecessor.

## Verification boundary

Repository-tool proof must copy the complete `.agents/skills/mcp-test` bundle,
including `scripts/scenario_core`, `scripts/scenario_flow`,
`scripts/scenario_compare`, and this skill. Invoke the copied CLI from outside
the source checkout with `PYTHONPATH` and `PYTHONHOME` absent. Check the JSON
`implementation` paths and `surface` value to confirm that the CLI and imported
producer modules come from the copied repository-tool bundle.

Source-only imports, direct producer calls, or synthetic fixture calls do not
prove the copied repository-tool surface. They are useful lower-level checks and
must remain separate from repository CLI evidence. The Devtools package must be
checked separately for absence of this bundle and retention of its Codegraph and
LSP commands.
