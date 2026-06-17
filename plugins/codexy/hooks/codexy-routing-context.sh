#!/bin/sh
set -eu

event="${1:-SessionStart}"
case "$event" in
  SessionStart) ;;
  *) event="SessionStart" ;;
esac

cat <<JSON
{"hookSpecificOutput":{"hookEventName":"$event","additionalContext":"Codexy plugin context: route Codexy work through \$codex-orchestration when applicable; keep non-trivial work issue-sized with real goal and plan state, use Codexy codegraph MCP when available, and run the packaged reviewer gate before PR readiness."}}
JSON
