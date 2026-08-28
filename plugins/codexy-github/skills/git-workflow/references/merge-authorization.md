# Authoritative Merge Authorization

Passing gates make a PR eligible; they do not authorize merge. The installed
canonical wrapper MUST fresh-read the exact repository, PR number, base, head,
and squash intent immediately before mutation.

Explicit authorization requires one fresh GitHub PR comment with immutable
comment identity and URL, authored by an `OWNER` or `MEMBER`, whose body exactly
matches the live target:

```text
AUTHORIZE SQUASH MERGE: PR #<number> BASE <base> HEAD <head>
```

The repository-contract alternative uses the same authenticated comment
requirements and this exact body:

```text
AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #<number> BASE <base> HEAD <head>
```

A stale head, wrong repository/PR/base, generic finish, local JSON, claimed
actor, parent prose, silence, gate success, or unauthenticated intent MUST be
rejected. Authorization MUST remain independent from checks, reviews, comments,
threads, labels, title, issue linkage, connector policy, merge-message
validation, cleanup, and post-merge proof.

Direct or nested `mcp__codex_apps__github_merge_pull_request` and auto-merge
connector calls remain `UNAVAILABLE`. The public installed wrapper accepts no
local authorization-state substitute; its fresh authenticated capture is the
only Codexy-owned authorization path.
