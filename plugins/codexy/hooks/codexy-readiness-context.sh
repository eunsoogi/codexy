#!/bin/sh
set -eu

event="${1:-UserPromptSubmit}"
case "$event" in
  UserPromptSubmit|SessionStart|Stop) ;;
  *) event="UserPromptSubmit" ;;
esac

printf '%s\n' "{\"hookSpecificOutput\":{\"hookEventName\":\"$event\",\"additionalContext\":\"Codexy readiness context: keep routing context separate from readiness and validation gates. For PR readiness, use explicit validator or workflow commands rather than relying on SessionStart context. PR label readiness enforcement (#210): before PR-ready or merge-ready claims, capture PR state with repositoryLabels and run hooks/codexy-readiness-guard.sh --check-pr-labels --pr-state-file pr-state.json; an unlabeled PR is blocked when repository labels exist. For completion handoff claims, also run scripts/validate-plugin-config --check-completion-handoff --handoff-file handoff.md --pr-state-file pr-state.json. PR title and merge subject enforcement (#206): run the packaged codexy-readiness-guard hook or the equivalent validator command before PR readiness or merge readiness. Preserve static packaged hook entrypoints, bounded timeouts, no user-state mutation, and validator-backed evidence before handoff.\"}}"
