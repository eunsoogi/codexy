# Authoritative Merge Authorization

Passing gates make a pull request eligible; they are not merge authorization.
Before `gh pr merge`, auto-merge, or an equivalent mutation, the actor captures
a fresh JSON record whose `kind` is `explicit-user-intent`,
`explicit-maintainer-intent`, or `repository-workflow-contract`. The record
uses `intent: "merge"`, `mergeClass: "squash"`, and the exact `prNumber`,
`baseRefName`, and `headRefOid` returned by GitHub immediately before mutation.

An explicit user or maintainer intent is authoritative when it has the matching
actor, an exact current pull-request target, a nonempty `*-intent://` source,
and `recordIssuer: "maintainer-recorded"`. The alternative checked record is
`repository-workflow-contract`, defined by `merge-authorization-contract.json`;
it carries that contract's exact ID, version, and target. Generic finish,
completion, silence, closing text, parent prose, gate success, ambiguity,
negation, and stale/wrong targets are non-authoritative signals. This
global invariant applies to every workflow profile. A gate-satisfied pull
request without the checked record remains open and waiting.

Authorization alone does not satisfy review, ownership, checks, labels, title,
connector, Sentinel, merge-message, cleanup, or post-merge synchronization
gates. Authorization and gate requirements remain in force with `--auto` and
`--admin`.

```bash
authorization_file="${AUTHORIZATION_FILE:?set AUTHORIZATION_FILE to the authorization JSON path}"
pr_state_file=$(mktemp)
trap 'rm -f "$pr_state_file"' EXIT

gh pr view "$pr_number" --repo "$repo" --json number,baseRefName,headRefOid > "$pr_state_file"
scripts/validate-plugin-config --check-merge-authorization \
  --merge-authorization-file "$authorization_file" \
  --merge-authorization-pr-state-file "$pr_state_file"
```

The command has to succeed before the separate canonical squash-merge command
in `merge-and-main-sync.md` is run.
