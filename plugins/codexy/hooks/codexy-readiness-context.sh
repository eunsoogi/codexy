#!/bin/sh
set -eu

event="${1:-UserPromptSubmit}"
case "$event" in
  UserPromptSubmit|SessionStart|Stop) ;;
  *) event="UserPromptSubmit" ;;
esac

printf '%s\n' "{\"hookSpecificOutput\":{\"hookEventName\":\"$event\",\"additionalContext\":\"Codexy readiness context: keep routing context separate from readiness and validation gates. For PR readiness, use separated hard hook modes rather than relying on SessionStart or UserPromptSubmit context alone. PR label readiness enforcement (#210): before PR-ready or merge-ready claims, capture PR state with repositoryLabels and run hooks/codexy-pr-label-check.sh --pr-state-file pr-state.json, which delegates the --check-pr-labels hard check; an unlabeled PR is blocked when repository labels exist. Keep scripts/validate-plugin-config --check-completion-handoff --handoff-file handoff.md --pr-state-file pr-state.json in the same PR-readiness path so linked issue labels and repositoryLabels evidence cannot be skipped after the label hook passes. PR title and merge subject enforcement (#206): run hooks/codexy-pr-title-check.sh --pr-title before PR readiness and hooks/codexy-merge-message-check.sh --expected-pr before merge readiness, or the equivalent validator commands. Before parent/orchestrator directives require hook entrypoints on a stacked child lane, validate those hook entrypoints against the child lane target base; if the target base lacks the hook path, name the available fallback validator command or record the mismatch as a separate dogfood defect instead of blocking the child on a future-branch path. Preserve static packaged hook entrypoints, bounded timeouts, no user-state mutation, and validator-backed evidence before handoff.\"}}"
