# Scenario format

`run_scenario.py` accepts one JSON object. The format keeps the execution
boundary visible and makes a dependent call use data selected by its
predecessor.

## Required shape

```json
{
  "id": "search-to-detail",
  "deadline_seconds": 10,
  "steps": [
    {
      "name": "search",
      "targets": {
        "installed": {
          "argv": ["/absolute/path/to/server", "--step", "search"],
          "cwd": "/absolute/path/to/workdir",
          "environment": {"SCENARIO_MARKER": "mcp-test"}
        }
      },
      "tool": "search",
      "allowed_tools": ["search", "detail"],
      "arguments": {"query": "codex"},
      "stored_fields": {"id": "/result/data/id"},
      "expected": {
        "kind": "success",
        "fields": {"/result/data/id": "item-42"}
      }
    },
    {
      "name": "detail",
      "targets": {
        "installed": {
          "argv": ["/absolute/path/to/server", "--step", "detail"],
          "cwd": "/absolute/path/to/workdir",
          "environment": {"SCENARIO_MARKER": "mcp-test"}
        }
      },
      "tool": "detail",
      "allowed_tools": ["search", "detail"],
      "arguments": {"id": null},
      "stored_fields": {"matched_id": "/result/data/matched_id"},
      "expected": {
        "kind": "success",
        "fields": {"/result/data/matched_id": "item-42"}
      },
      "references": {
        "/id": {"step": "search", "path": "/id", "type": "string"}
      }
    }
  ]
}
```

Replace the target `argv` and `cwd` with an explicitly selected trusted local
stdio server and an existing absolute working directory. Every step must
declare the same target names. A target may use an absolute interpreter path
and a server script as separate `argv` entries; shell quoting and interpolation
are not supported.

## Step fields

- `tool` is the one requested tool and must be a member of `allowed_tools`.
- `arguments` is the initial JSON object. A reference target such as `/id` must
  already exist in this object so the flow can replace it safely.
- `stored_fields` maps a stable name to a non-root JSON Pointer. Only these
  values cross a step boundary or appear in reports.
- `expected.kind` is one of the producer result kinds, normally `success` or a
  deliberately expected JSON-RPC/tool error. `expected.fields` adds exact JSON
  Pointer assertions.
- `references` may point only to an earlier step. `type` may be `string`,
  `integer`, `number`, `boolean`, `object`, or `array`.

Optional step fields are `protocol_version`, `transport`, `timeout_seconds`,
and `output_limit_bytes`. The supported values remain explicit in the support
contract; an unsupported protocol or transport is rejected rather than
downgraded.

## Comparing two targets

Declare `baseline` and `candidate` target names in every step, then invoke:

```sh
python3 <devtools-root>/skills/mcp-test/scripts/run_scenario.py compare \
  --scenario scenario.json --baseline-target baseline \
  --candidate-target candidate
```

The comparison uses the same step order, tool allowlist, arguments,
expectations, stored fields, and references for both targets. Target-specific
`argv`, `cwd`, and environment are the only execution selection. Add a
`normalizers` array only for a named step/field whose comparison policy is
explicitly constant:

```json
"normalizers": [
  {"step": "search", "field": "timestamp",
   "normalizer": "constant", "value": "<normalized>"}
]
```

The CLI never treats matching failures as a pass and never serializes raw
process output or unselected server fields.
