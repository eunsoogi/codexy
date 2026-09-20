---
name: codegraph
description: Use when Codexy Devtools is installed and the task needs bounded repository structure, code search, import or dependency navigation, or Codegraph exploration.
---

# Codegraph

Use the packaged `codegraph` MCP for bounded repository exploration when it is
callable, then confirm the exact files and claims with direct reads. Choose the
tool that matches the question: `codegraph_overview` or `codegraph_index` for
structure and edges, `codegraph_search` for bounded search,
`codegraph_neighbors` or `codegraph_reverse_deps` for imports, and
`codegraph_neighborhood` for a bounded dependency neighborhood.

For a change-scoped question, call `codegraph_change_impact` with `root` and
either both `base` and `head` revisions or neither (working-tree mode). The
optional `maxFiles` and `maxPaths` values are returned in `impact.limits`.
Impact analysis follows Python and Rust dependency paths; unsupported languages,
unresolved imports, parse failures, and limit truncation remain explicit in
`impact.limits.unknown` and `partial`.

For advisory verification planning, call `codegraph_check_selection` with the
same change arguments plus explicit `mappings`. A mapping contains `owner`
(`user` or `repository`), `kind` (`path`, `shared_configuration`, or `fixture`),
`pattern`, `checkIds`, and a human-readable `reason`; its `checks` array
defines each check id, command text, and description. The response keeps
recommendation paths, reasons, gaps, broader verification, manual judgment,
and limits visible. It never runs a command, waives a check, or decides that
the change is complete.

For example, a commit-scoped recommendation request can use:

```json
{
  "base": "HEAD~1",
  "head": "HEAD",
  "mappings": {
    "checks": [{"id": "unit", "command": "cargo test", "description": "unit tests"}],
    "mappings": [{"owner": "repository", "kind": "path", "pattern": "src/**", "checkIds": ["unit"], "reason": "source changes need unit coverage"}]
  },
  "dependencyState": "unconfirmed"
}
```

The three installed-runtime demonstrations in
`packages/codexy-runtime/tests/mcp_stdio/change_impact/` cover documentation,
a single module, and a shared fixture. Each verifies recommendations, reasons,
limits, and the non-execution proof boundary. These subprocess tests prove the
installed wrapper and bundled runtime only; active host exposure is
`unobserved` until the host's callable tool list and an invocation are checked
separately.

Keep `root` and every path inside the user-authorized workspace. Respect each
tool's limits and preserve returned `partial`, `errors`, and truncation
metadata. If the MCP, root, source, or result is unavailable, missing,
unreadable, or partial, record that status and use a proportional direct-read
fallback; tool configuration is not proof that a call succeeded.

This package is optional, so core Codexy workflows MUST remain usable without
it. Codegraph-only work MUST NOT initialize or call an LSP server. Command
overrides remain disabled unless the user explicitly authorizes the supported
opt-in.
