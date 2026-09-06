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

Keep `root` and every path inside the user-authorized workspace. Respect each
tool's limits and preserve returned `partial`, `errors`, and truncation
metadata. If the MCP, root, source, or result is unavailable, missing,
unreadable, or partial, record that status and use a proportional direct-read
fallback; tool configuration is not proof that a call succeeded.

This package is optional, so core Codexy workflows MUST remain usable without
it. Codegraph-only work MUST NOT initialize or call an LSP server. Command
overrides remain disabled unless the user explicitly authorizes the supported
opt-in.
