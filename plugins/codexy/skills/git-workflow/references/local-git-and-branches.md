# Local Git And Branches

## Worktrees And Branches

MUST create task worktrees from an up-to-date `main`:

```sh
git fetch origin main
git switch main
git pull --ff-only origin main
git worktree add -b codexy/<issue-or-scope> ../<repo>-worktrees/<issue-or-scope> main
```

MUST NOT force-push task branches. If push is rejected because the remote branch
changed, MUST inspect the remote changes and bring required adjustments in with a
new commit.

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
`revert`. Project-local skill changes under `plugins/codexy/skills/**` change
agent behavior, so prefer non-`docs` types. MUST NOT use vague messages such as
`update`, `fix`, `WIP`, or `misc`.

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
