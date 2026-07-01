#!/bin/sh
set -eu

event="${1:-SessionStart}"
case "$event" in
  SessionStart) ;;
  *) event="SessionStart" ;;
esac

cat <<JSON
{"hookSpecificOutput":{"hookEventName":"$event","additionalContext":"Codexy plugin context: route Codexy work through \$codex-orchestration when applicable; keep non-trivial work issue-sized with real goal and plan state. For compacted or resumed context hygiene, use \$dreaming before continuing so stale summary details are separated from active facts. Use Codexy codegraph MCP before direct file reads when callable; include codegraph findings in handoff or PR readiness evidence, or record codegraph unavailable/uncallable fallback evidence and registered-but-uncallable/unavailable-tool evidence. Use Codexy LSP for language-aware code edits when a matching server is registered and callable; run lsp_status or record unavailable/not applicable evidence when the server is not usable. Run scripts/validate-plugin-config --check-touched-loc --base-ref origin/main for code or test-harness changes, and run the packaged reviewer gate before PR readiness. Before PR readiness, run hooks/codexy-readiness-guard.sh --check-pr-title with the exact PR title; before merge readiness, run hooks/codexy-readiness-guard.sh --check-merge-message with the explicit squash merge message."}}
JSON
