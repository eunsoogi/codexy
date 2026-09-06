---
name: lsp
description: Use when Codexy Devtools is installed and the task needs language-aware diagnostics, symbols, definitions, references, or bounded LSP requests.
---

# LSP

Use `lsp_status` or `lsp_for_path` to confirm the matching configured server
and its actual availability before relying on a language-aware result. Use
`lsp_document_symbols`, `lsp_definition`, `lsp_references`, or
`lsp_diagnostics` for one request. Paths must stay inside the user-authorized
workspace; keep repository `root` and the server's canonical `workspaceRoot`
distinct when both are supplied. An unavailable server returns an explicit
unavailable result with its reason and install hints; configuration alone is
not proof of an actual call.

For several requests in one workspace, use the additive `lsp_batch` tool. Put
`root`, `workspaceRoot`, `server`, `timeoutMs`, and an optional `deadlineMs` at
the batch level; each request supplies one full `lsp_*` method and `path`, with
positions or `includeDeclaration` when needed. A batch contains 1–8 requests,
uses one server session and one canonical workspace, clamps request timeouts to
100–60,000 ms, and caps its deadline at 60 seconds. It returns one ordered
result per request. Per-file errors continue to later independent requests;
transport or session failure stops remaining work, and cleanup still runs
within the deadline. Workspace-readiness or stderr failures must remain
explicit errors rather than being reported as successful diagnostics.

This package is optional, so core Codexy workflows MUST remain usable without
it. LSP-only work MUST NOT require a Codegraph sweep. Command overrides remain
disabled unless the user explicitly authorizes the supported opt-in.
