---
name: developer-tools
description: Use when Codexy Devtools is installed and the task needs local Codegraph exploration or LSP diagnostics.
---

# Codexy Devtools

Use the packaged `codegraph` MCP for repository exploration when it is callable,
then confirm exact files with direct reads. Use the packaged `lsp` MCP for
language-aware checks when it is callable. If either tool is unavailable, record
that status and use a proportional direct-read or static-analysis fallback.

The devtools package is optional. Core Codexy workflows MUST remain usable when
this package is not installed. MCP requests and LSP file paths MUST stay within
the user-authorized workspace, and command overrides MUST remain disabled unless
the user explicitly authorizes the supported opt-in.

For several requests in one workspace, use the additive `lsp_batch` tool. Put
`root`, `workspaceRoot`, `server`, `timeoutMs`, and an optional `deadlineMs` at
the batch level; each item supplies one full `lsp_*` method and `path`. Batches
contain at most eight items, use one server session, and return one ordered
result per item. The batch deadline is capped at 60 seconds, and a server
failure marks the active and remaining items explicitly.
