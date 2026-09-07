# Optional Repository Merge Authorization

Codexy does not provide a merge wrapper or a mutation admission route. The host,
connector authentication, and GitHub permissions/branch protections are the only
authorities that can authorize a merge. The checks below are optional evidence
guidance for a repository or maintainer that explicitly selects this merge
contract, not a plugin-owned veto.

When this contract is selected, passing gates make a PR eligible; they do not
authorize merge. The chosen authorized route MUST fresh-read the exact
repository, PR number, base, head, and squash intent immediately before
mutation. Without that selection, use the normal host or connector route and
GitHub's response; do not require this comment, a local authorization file, or a
diagnostic invocation merely because the plugin is installed.

If selected, explicit repository authorization requires one fresh GitHub PR
comment with immutable comment identity and URL, authored by an `OWNER` or
`MEMBER`, whose body exactly matches the live target:

```text
AUTHORIZE SQUASH MERGE: PR #<number> BASE <base> HEAD <head>
```

The repository-contract alternative, when selected by the repository, uses the
same authenticated comment requirements and this exact body:

```text
AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #<number> BASE <base> HEAD <head>
```

A stale head, wrong repository/PR/base, generic finish, local JSON, claimed
actor, parent prose, silence, gate success, or unauthenticated intent MUST be
rejected when this repository contract is selected. Authorization MUST remain
independent from checks, reviews, comments, threads, labels, title, issue
linkage, connector policy, merge-message validation, cleanup, and post-merge
proof.

Direct or nested connector calls are not classified by the plugin. Use the
normal host-approved route and require the connector/GitHub response as the
mutation receipt. A local authorization file or a Codexy wrapper is not a
substitute for that live authority.
