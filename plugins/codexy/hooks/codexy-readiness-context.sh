#!/bin/sh
set -eu

event="${1:-UserPromptSubmit}"
case "$event" in
  UserPromptSubmit|SessionStart|Stop) ;;
  *) event="UserPromptSubmit" ;;
esac

cat <<JSON
{"hookSpecificOutput":{"hookEventName":"$event","additionalContext":"Codexy readiness context: keep routing context separate from readiness and validation gates. For PR readiness, use explicit validator or workflow commands rather than relying on SessionStart context. Future PR title and merge subject enforcement (#206) can attach here through a dedicated packaged hook or readiness command. Future PR label readiness enforcement (#210) can attach here through a dedicated packaged hook or readiness command. Preserve static packaged hook entrypoints, bounded timeouts, no user-state mutation, and validator-backed evidence before handoff."}}
JSON
