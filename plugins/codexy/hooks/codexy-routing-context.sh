#!/bin/sh
set -eu

event="${1:-SessionStart}"
case "$event" in
  SessionStart) ;;
  *) event="SessionStart" ;;
esac

printf '%s\n' "{\"hookSpecificOutput\":{\"hookEventName\":\"$event\",\"additionalContext\":\"Codexy plugin context: route Codexy work through \$codex-orchestration when applicable; keep non-trivial work issue-sized with real goal and plan state. For compacted or resumed context hygiene, use \$dreaming before continuing so stale summary details are separated from active facts. Use Codexy codegraph MCP before direct file reads when callable; include codegraph findings in handoff or PR readiness evidence, or record codegraph unavailable/uncallable fallback evidence and registered-but-uncallable/unavailable-tool evidence. Use Codexy LSP for language-aware code edits when a matching server is registered and callable; run lsp_status or record unavailable/not applicable evidence when the server is not usable. Run scripts/validate-plugin-config --check-touched-loc --base-ref origin/main for code or test-harness changes. Before PR-ready or merge-ready claims, capture PR state with repositoryLabels and run both hooks/codexy-pr-label-check.sh --pr-state-file pr-state.json and scripts/validate-plugin-config --check-completion-handoff --handoff-file handoff.md --pr-state-file pr-state.json; an unlabeled PR is blocked when repository labels exist, and the validator keeps linked issue labels/repositoryLabels evidence in the readiness path. Before PR readiness, run hooks/codexy-pr-title-check.sh --pr-title with the exact PR title; before merge readiness, run hooks/codexy-merge-message-check.sh --expected-pr PR_NUMBER with the explicit squash merge message. These hard hook modes correspond to --check-pr-title, --check-pr-labels, and --check-merge-message. Run the packaged reviewer gate before PR readiness.\"}}"
