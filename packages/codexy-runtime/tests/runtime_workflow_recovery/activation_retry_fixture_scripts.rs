pub(super) const GH: &str = r#"#!/bin/sh
set -eu
test "${GH_TOKEN:-}" = fixture-github-token || { echo "verifier GitHub query requires GH_TOKEN: $CODEXY_FIXTURE_STEP" >&2; exit 4; }
case "$*" in
  'pr list '*'--json number,headRefName'*)
    if case "$*" in *'--head '*) true ;; *) false ;; esac; then
      if test "$PR_READBACK_MODE" = wrong-base; then
        printf '[{"number":1,"headRefName":"%s","headRefOid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","baseRefName":"release","baseRefOid":"%s","isCrossRepository":false,"headRepository":{"nameWithOwner":"eunsoogi/codexy"},"headRepositoryOwner":{"login":"eunsoogi"}}]\n' "$OPEN_ACTIVATION_BRANCH" "$(git rev-parse main)"
      elif test "$PR_READBACK_MODE" = readback-duplicate && test -f "$PR_READBACK_ATTEMPT"; then
        head="$(git rev-parse HEAD)"; base="$(git rev-parse main)"
        printf '[{"number":1,"headRefName":"%s","headRefOid":"%s","baseRefName":"main","baseRefOid":"%s","isCrossRepository":false,"headRepository":{"nameWithOwner":"eunsoogi/codexy"},"headRepositoryOwner":{"login":"eunsoogi"}},{"number":2,"headRefName":"%s","headRefOid":"%s","baseRefName":"main","baseRefOid":"%s","isCrossRepository":false,"headRepository":{"nameWithOwner":"eunsoogi/codexy"},"headRepositoryOwner":{"login":"eunsoogi"}}]\n' "$ACTIVATION_BRANCH" "$head" "$base" "$ACTIVATION_BRANCH" "$head" "$base"
      elif test "$PR_READBACK_MODE" = readback-missing && test -f "$PR_READBACK_ATTEMPT"; then
        printf '[]\n'
      elif test "$(cat "$PR_STATE_FILE")" = 1; then
        head="$(git rev-parse HEAD)"; base="$(git rev-parse main)"
        if test "$CODEXY_FIXTURE_STEP" = "Create exactly one activation pull request"; then
          if test ! -f "$PR_READBACK_ATTEMPT"; then
            printf '%s' 0 > "$PR_READBACK_ATTEMPT"
          else
            attempt=$(cat "$PR_READBACK_ATTEMPT")
            attempt=$((attempt + 1)); printf '%s' "$attempt" > "$PR_READBACK_ATTEMPT"
            if test "$PR_READBACK_MODE" = readback-wrong || test "$attempt" -lt 3; then head="$(git rev-parse HEAD^)"; fi
          fi
        fi
        printf '[{"number":1,"headRefName":"%s","headRefOid":"%s","baseRefName":"main","baseRefOid":"%s","isCrossRepository":false,"headRepository":{"nameWithOwner":"eunsoogi/codexy"},"headRepositoryOwner":{"login":"eunsoogi"}}]\n' "$ACTIVATION_BRANCH" "$head" "$base"
      else
        printf '[]\n'
      fi
    elif test "$PR_READBACK_MODE" = saturated; then
      printf '['
      index=1
      while test "$index" -le 101; do
        test "$index" -eq 1 || printf ','
        printf '{"number":%s,"headRefName":"codexy/runtime-activation-v1.7.0-other-%s","headRefOid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","baseRefName":"main","baseRefOid":"%s","isCrossRepository":false,"headRepository":{"nameWithOwner":"eunsoogi/codexy"},"headRepositoryOwner":{"login":"eunsoogi"}}' "$index" "$index" "$(git rev-parse main)"
        index=$((index + 1))
      done
      printf ']\n'
    elif test "$PR_READBACK_MODE" = fork-competing; then
      printf '[{"number":7,"headRefName":"%s","headRefOid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","baseRefName":"main","baseRefOid":"%s","isCrossRepository":true,"headRepository":{"nameWithOwner":"external/codexy"},"headRepositoryOwner":{"login":"external"}}]\n' "$OPEN_ACTIVATION_BRANCH" "$(git rev-parse main)"
    elif test "$PR_READBACK_MODE" = wrong-base; then
      printf '[]\n'
    elif test "$(cat "$PR_STATE_FILE")" = 1; then
      printf '[{"number":1,"headRefName":"%s","headRefOid":"%s","baseRefName":"main","baseRefOid":"%s","isCrossRepository":false,"headRepository":{"nameWithOwner":"eunsoogi/codexy"},"headRepositoryOwner":{"login":"eunsoogi"}}]\n' "$OPEN_ACTIVATION_BRANCH" "$(git rev-parse HEAD)" "$(git rev-parse main)"
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
