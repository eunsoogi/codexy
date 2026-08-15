# Local Git And Branches

## Worktrees And Branches

MUST discover the repository default branch and create task worktrees from its
up-to-date remote-tracking branch:

```sh
default_branch=$(gh repo view --json defaultBranchRef --jq .defaultBranchRef.name)
git fetch origin "$default_branch"
git switch "$default_branch"
git pull --ff-only origin "$default_branch"
git worktree add -b <policy-prefix><issue-or-scope> ../<repo>-worktrees/<issue-or-scope> "$default_branch"
```

MUST NOT force-push task branches. If push is rejected because the remote branch
changed, MUST inspect the remote changes and bring required adjustments in with
a new commit.

## Local Change Discipline

MUST inspect before editing or committing:

```sh
git status --short
git diff
```

MUST stage only intended files. MUST preserve unrelated dirty work. MUST NOT
revert or discard user changes unless explicitly asked. MUST NOT commit
`.omo/**`, local logs, secrets, or scratch files by default.

## Commit Messages

MUST use Conventional Commit style:

```text
<type>(<scope>): <summary>
```

Common types are `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `ci`, and
`revert`. Installed plugin skill changes change agent behavior, so prefer
non-`docs` types. MUST NOT use vague messages such as `update`, `fix`, `WIP`, or
`misc`.

## Conflict Resolution

Before resolving conflicts, MUST inspect:

```sh
git status
git diff
```

MUST resolve conflict markers carefully. MUST preserve both sides' intended
behavior when possible. If resolution depends on domain intent, MUST stop and
ask. After resolving, MUST stage only resolved files and run relevant
verification.
