# Authoritative Merge Authorization

Passing gates make a pull request eligible; they are not merge authorization.
Before `gh pr merge`, auto-merge, or an equivalent mutation, the installed
canonical wrapper captures the exact target PR directly from GitHub immediately
before validation. It derives its ephemeral JSON record whose `kind` is
`explicit-user-intent`, `explicit-maintainer-intent`, or
`repository-workflow-contract`. The record uses `intent: "merge"`,
`mergeClass: "squash"`, and the exact `prNumber`, `baseRefName`, and
`headRefOid` returned by GitHub immediately before mutation.

The native connector merge and auto-merge tools are not an authorization path.
When nested connector calls do not carry authenticated hook-event coverage, they
are unavailable and MUST be routed through the existing
`hooks/codexy-authorized-squash-merge.sh` wrapper. The wrapper's fresh capture is
the only supported fallback; do not infer coverage from a nested producer or
parse its `functions.exec` source.

An explicit user or maintainer intent is authoritative only when its record
references one fresh GitHub PR comment with the immutable `commentId` and
`commentUrl`, authored by an `OWNER` or `MEMBER`. Its body is exactly
`AUTHORIZE SQUASH MERGE: PR #<number> BASE <base> HEAD <head>` for the current
PR state; arbitrary schemes, claimed actors, and parent prose MUST NOT count as
authorization. The alternative checked record is `repository-workflow-contract`;
it MUST cite one fresh OWNER or MEMBER GitHub PR comment with immutable
`contractCommentId` and `contractCommentUrl`. Its body is exactly
`AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #<number> BASE <base> HEAD <head>`.
Repository-local files, claimed issuers, IDs, and versions are not
authoritative. Generic finish, completion, silence, closing text, parent prose,
gate success, ambiguity, negation, and stale/wrong targets are non-authoritative
signals. This global invariant applies to every workflow profile. A
gate-satisfied pull request without the checked record remains open and waiting.

Authorization alone does not satisfy review, ownership, checks, labels, title,
connector, selected-profile review, merge-message, cleanup, or post-merge
synchronization gates. Authorization and gate requirements remain in force with
`--auto` and `--admin`.

The public wrapper has no `--merge-authorization-file` or
`--merge-authorization-pr-state-file` inputs. Repository-local JSON serves
validator test input only; installed merge mutations derive authority solely
from the wrapper's fresh internal GitHub capture. The wrapper keeps the capture
and derived record ephemeral, binding repository, PR, head, comment identity,
and association before it can run `gh pr merge`.
