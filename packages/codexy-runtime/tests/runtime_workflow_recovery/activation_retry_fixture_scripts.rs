pub(super) const GH: &str = r#"#!/bin/sh
set -eu
test "${GH_TOKEN:-}" = fixture-github-token || { echo "verifier GitHub query requires GH_TOKEN: $CODEXY_FIXTURE_STEP" >&2; exit 4; }
case "$*" in
  'pr list '*'--json number,headRefName'*)
    if test "$PR_READBACK_MODE" = saturated; then
      printf '['
      index=1
      while test "$index" -le 101; do
        test "$index" -eq 1 || printf ','
        printf '{"number":%s,"headRefName":"codexy/runtime-activation-v1.7.0-other-%s"}' "$index" "$index"
        index=$((index + 1))
      done
      printf ']\n'
    elif test "$(cat "$PR_STATE_FILE")" = 1; then
      printf '[{"number":1,"headRefName":"%s"}]\n' "$OPEN_ACTIVATION_BRANCH"
    else
      printf '[]\n'
    fi
    ;;
  'pr list '*'--json state '*)
    case "$*" in
      *"--head $LEGACY_BRANCH "*)
        if test -n "$LEGACY_PR_STATE"; then
          printf '%s\n' "$LEGACY_PR_STATE"
        elif test "$(cat "$PR_STATE_FILE")" = 1; then
          printf '%s\n' OPEN
        fi
        ;;
      *) if test "$(cat "$PR_STATE_FILE")" = 1; then printf '%s\n' OPEN; fi ;;
    esac
    ;;
  'pr list '*'--json number '*) cat "$PR_STATE_FILE" ;;
  'pr list '*'--json headRefOid '*)
    attempt=0
    test ! -f "$PR_READBACK_ATTEMPT" || attempt=$(cat "$PR_READBACK_ATTEMPT")
    attempt=$((attempt + 1)); printf '%s' "$attempt" > "$PR_READBACK_ATTEMPT"
    head=$(git rev-parse HEAD); count=$(cat "$PR_STATE_FILE")
    case "$PR_READBACK_MODE" in
      readback-delay) if test "$attempt" -lt 3; then head=$(git rev-parse HEAD^); fi ;;
      readback-wrong) head=$(git rev-parse HEAD^) ;;
      readback-missing) head=missing; count=0 ;;
      readback-duplicate) count=2 ;;
    esac
    case "$*" in *'@tsv'*) printf '%s\t%s\n' "$count" "$head" ;; *) printf '%s\n' "$head" ;; esac
    ;;
  'pr create '*) test "$(cat "$PR_STATE_FILE")" = 0; printf 1 > "$PR_STATE_FILE" ;;
  *) echo "unexpected GitHub mutation: $*" >&2; exit 98 ;;
esac
"#;

pub(super) const CARGO: &str = r#"#!/bin/sh
set -eu
case "$*" in *'--bin codexy-sync-version -- '*) ;; *) exit 99 ;;
esac
while test "$1" != --; do shift; done
shift
exec "$CODEXY_TEST_SYNC_VERSION_BINARY" "$@"
"#;
