#!/bin/sh
# Admit the exact squash merge, then execute it through the installed hook path.
set -efu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
expected_pr=
expected_issue=
message_file=
repo=
head_oid=
subject=
body_file=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --expected-pr) expected_pr=${2-}; shift 2 ;;
    --expected-issue) expected_issue=${2-}; shift 2 ;;
    --merge-message-file) message_file=${2-}; shift 2 ;;
    --repo) repo=${2-}; shift 2 ;;
    --match-head-commit) head_oid=${2-}; shift 2 ;;
    --subject) subject=${2-}; shift 2 ;;
    --body-file) body_file=${2-}; shift 2 ;;
    *) printf '%s\n' "unsupported authorized merge argument: $1" >&2; exit 2 ;;
  esac
done

for required in "$expected_pr" "$message_file" "$repo" "$head_oid" "$subject" "$body_file"; do
  [ -n "$required" ] || { printf '%s\n' 'missing required authorized merge argument' >&2; exit 2; }
done

owner=${repo%%/*}
name=${repo#*/}
[ "$owner/$name" = "$repo" ] || { printf '%s\n' 'repo must be owner/name' >&2; exit 2; }
authorization_file=$(mktemp)
pr_state_file=$(mktemp)
merge_body_file=$(mktemp)
merge_payload_file=$(mktemp)
trap 'rm -f "$authorization_file" "$pr_state_file" "$merge_body_file" "$merge_payload_file"' EXIT
cat < "$body_file" > "$merge_body_file"
printf '%s\n\n' "$subject" > "$merge_payload_file"
cat < "$merge_body_file" >> "$merge_payload_file"
if ! cmp -s "$message_file" "$merge_payload_file"; then
  printf '%s\n' 'merge message file does not match subject and body payload' >&2
  exit 1
fi
if ! gh api graphql --paginate --slurp -f owner="$owner" -f name="$name" -F number="$expected_pr" -f query='
query($owner:String!, $name:String!, $number:Int!, $endCursor:String) {
  repository(owner:$owner, name:$name) { nameWithOwner pullRequest(number:$number) {
    number baseRefName headRefOid comments(first:100, after:$endCursor) {
      nodes { id url body author { login } authorAssociation }
      pageInfo { hasNextPage endCursor }
    }
  }}
}' > "$pr_state_file"; then
  printf '%s\n' 'failed to capture current GitHub PR authorization state' >&2
  exit 1
fi
python3 -I -B "$script_dir/codexy-github-merge-authorization.py" \
  --repo "$repo" --expected-pr "$expected_pr" --expected-head "$head_oid" \
  --pr-state-file "$pr_state_file" --authorization-file "$authorization_file"

set -- --expected-pr "$expected_pr" --merge-message-file "$merge_payload_file"
[ -z "$expected_issue" ] || set -- "$@" --expected-issue "$expected_issue"
"$script_dir/codexy-merge-admission-check.sh" "$@" \
  --merge-authorization-file "$authorization_file" \
  --merge-authorization-pr-state-file "$pr_state_file"
if gh pr merge "$expected_pr" --repo "$repo" --squash --delete-branch \
  --match-head-commit "$head_oid" --subject "$subject" --body-file "$merge_body_file"; then
  merge_status=0
else
  merge_status=$?
fi
rm -f "$authorization_file" "$pr_state_file" "$merge_body_file" "$merge_payload_file"
trap - EXIT
exit "$merge_status"
