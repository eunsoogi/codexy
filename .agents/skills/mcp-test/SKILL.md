---
name: mcp-test
description: Use when working in the Codexy repository and you need to run or compare explicit local MCP scenarios with search-to-detail chaining, expectations, or selected-field normalization.
---

# MCP Test

Use `.agents/skills/mcp-test/scripts/run_scenario.py` from a Codexy checkout to
exercise a declarative MCP scenario with the repository's development tooling.
The CLI calls the repository-local scenario core, flow, and comparison
producers; it is not a single-call wrapper or an installed end-user workflow.

## Commands

```sh
python3 <codexy-root>/.agents/skills/mcp-test/scripts/run_scenario.py support
python3 <codexy-root>/.agents/skills/mcp-test/scripts/run_scenario.py run \
  --scenario <scenario.json> --target installed
python3 <codexy-root>/.agents/skills/mcp-test/scripts/run_scenario.py compare \
  --scenario <scenario.json> --baseline-target baseline \
  --candidate-target candidate
```

`run` returns exit code `0` only when every step meets its expectation. A
failed, malformed, unsupported, or incomparable execution returns `2`. `compare`
returns `0` for a match, `1` for a behavioral difference, and `2` when either
run fails or cannot be compared. Output is JSON and contains only selected
stored fields, bounded errors, step failures, and implementation provenance.

## Scenario boundary

The scenario file must declare an `id`, ordered `steps`, and the same explicit
target names for every step. Each step declares `tool`, `allowed_tools`,
`stored_fields`, optional `arguments`, `expected`, and optional `references`.
Each target declares an `argv`, an existing absolute `cwd`, and an explicit
environment mapping. Commands are launched with `shell=False`; response data is
inert data and never becomes a command or environment value.

Use a reference to carry a selected value from an earlier step into a declared
argument, for example:

```json
"arguments": {"id": null},
"references": {
  "/id": {"step": "search", "path": "/id", "type": "string"}
}
```

The complete search-to-detail manifest is in `references/scenario-format.md`. It
proves that the returned search identifier, not a hard-coded detail value,
drives the dependent call.

## Comparison policy

The baseline and candidate targets receive the same ordered scenario contract.
Differences are reported by scenario, step, selected field, error, or linkage.
No field is normalized implicitly. To normalize a known nondeterministic field,
declare a constant explicitly in the manifest:

```json
"normalizers": [
  {"step": "search", "field": "timestamp",
   "normalizer": "constant", "value": "<normalized>"}
]
```

The support and CI boundary is documented in `references/support-contract.md`.
The initial contract is trusted local newline-delimited stdio, protocol
`2024-11-05`, and POSIX only. Windows is rejected before launch until native
descendant ownership is proven. Remote authenticated connections, HTTP
transports, ambient environment inheritance, and host/app skill-call claims are
outside this CLI.

Run the CLI from the repository root or any other directory. The report's
`implementation` object must point at the repository-only CLI and producer
modules under `.agents/skills/mcp-test`, and its `surface` must be
`repository-only`. These subprocess results prove repository-tool provenance
only; they do not prove an installed package or an active host/app skill
surface.
