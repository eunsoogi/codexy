#!/bin/sh
set -eu

state=${FIXTURE_STATE:?}

argument_after() {
  needle=$1
  shift
  while [ "$#" -gt 1 ]; do
    if [ "$1" = "$needle" ]; then
      printf '%s\n' "$2"
      return 0
    fi
    shift
  done
  return 1
}

record_mutation() {
  printf '%s\n' "$1" >> "$state/mutations.log"
  printf 'mutated\n' > "$state/mutation-sentinel"
}

command=${1:-}
shift || true
case "$command" in
  issue)
    cat "$state/issue.json"
    ;;
  label)
    cat "$state/labels.json"
    ;;
  api)
    if [ "${1:-}" = graphql ]; then
      cat "$state/review-threads.json"
    elif printf '%s\n' "$*" | grep -q -- '--method GET'; then
      cat "$state/existing-prs.json"
    elif printf '%s\n' "$*" | grep -q -- '--method PUT'; then
      record_mutation label-put
      cat "$state/labels.json"
    else
      printf 'unsupported gh api: %s\n' "$*" >&2
      exit 2
    fi
    ;;
  pr)
    subcommand=${1:-}
    shift || true
    case "$subcommand" in
      create)
        record_mutation pr-create
        body_file=$(argument_after --body-file "$@")
        cp "$body_file" "$state/current-body.md"
        printf 'https://github.com/eunsoogi/codexy/pull/999\n'
        ;;
      edit)
        record_mutation pr-edit
        body_file=$(argument_after --body-file "$@")
        cp "$body_file" "$state/current-body.md"
        ;;
      view)
        if printf '%s\n' "$*" | grep -q -- '--jq .number'; then
          printf '999\n'
        elif printf '%s\n' "$*" | grep -q -- '--json number,body,headRefName,headRefOid,labels,closingIssuesReferences'; then
          cat "$state/observed-pr.json"
        else
          oid=$(git rev-parse origin/codexy/version-1.3.1 2>/dev/null || git rev-parse HEAD)
          jq -n --rawfile body "$state/current-body.md" --arg oid "$oid" '{
            number:999,state:"OPEN",isDraft:false,mergeStateStatus:"CLEAN",reviewDecision:"",
            baseRefName:"main",body:$body,headRefName:"codexy/version-1.3.1",headRefOid:$oid,
            headRepository:{name:"codexy"},headRepositoryOwner:{login:"eunsoogi"},
            isCrossRepository:false,url:"https://github.com/eunsoogi/codexy/pull/999",
            labels:[{name:"priority/medium"},{name:"status/review"},{name:"type/ci"},{name:"area/release"}],
            closingIssuesReferences:[{number:301,url:"https://github.com/eunsoogi/codexy/issues/301"}]
          }'
        fi
        ;;
      *)
        printf 'unsupported gh pr: %s %s\n' "$subcommand" "$*" >&2
        exit 2
        ;;
    esac
    ;;
  *)
    printf 'unsupported gh command: %s %s\n' "$command" "$*" >&2
    exit 2
    ;;
esac
