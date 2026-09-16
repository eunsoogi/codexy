# Local plan storage

## Mode

MUST save or update a plan when the request asks to create, update, keep, or
save it, unless the user explicitly requests read-only or output-only behavior.
A plain `$planning` request MUST use save/update by default. Status-only
questions and output-only requests MUST NOT write.

## Destination precedence

MUST select a destination in this order:

1. The user's exact path when one is supplied.
2. Otherwise, one existing active plan for the same task or topic.
3. Otherwise, the default path:
   - Git project: repository root `.plans/<topic>.md`.
   - Non-Git directory: current working directory `.plans/<topic>.md`.

MUST use a safe, stable `<topic>` filename without path separators. MUST
preserve an explicit user path as given. If the target path already belongs to
another topic, or more than one active same-topic plan exists, MUST stop before
writing and MUST ask which path to use. A completed or archived plan is not an
active plan; MUST NOT reopen or overwrite it automatically.

## Git exclusion

For a Git project, MUST determine the repository root and the real
repository-local exclude path before creating an untracked default plan:

```sh
repo_root="$(git rev-parse --show-toplevel)"
git -C "$repo_root" rev-parse --git-path info/exclude
git -C "$repo_root" config --path --get core.excludesFile
git -C "$repo_root" check-ignore -v --no-index -- .plans/<topic>.md
```

MUST resolve the plan target relative to `repo_root`, and MUST run all
target-relative checks with `git -C "$repo_root"`; a bare `git check-ignore` can
inspect the caller's directory instead. MUST use the exact
`git rev-parse --git-path info/exclude` result, resolved to an absolute path,
rather than assuming `.git` is a directory. This supports a `.git` file and
linked worktree. MUST read existing `.gitignore`, the returned repository
exclude file, and any configured global exclude only as needed; MUST preserve
every existing line.

If `/.plans/` is not already an effective rule, MUST append that one rule to the
repository-local exclude file before saving the default untracked plan. MUST NOT
add a duplicate rule, modify a shared `.gitignore` just for a local plan, or
change a tracked plan's status. If the exclude file cannot be read or written,
MUST report `exclude not applied` and MUST NOT claim that the plan is ignored.

For a tracked or shared plan, MUST keep the user's selected path and MUST NOT
change ignore rules. For a non-Git directory, MUST save under its cwd `.plans`
path and MUST report that Git exclusion is not applicable.

## Safe update

MUST read the target before writing. MUST preserve unrelated headings, user
edits, completed or archived status, and every other plan file. MUST replace
only the same-topic active plan fields covered by the request. MUST NOT
overwrite user-authored lines; when a user edit conflicts with the update, MUST
preserve the original, MUST return the new plan as output, and MUST report the
conflict. If no safe boundary can be identified, MUST leave the file unchanged,
MUST return the new plan as output, and MUST report that it was not saved.

## Verification scenarios

When storage behavior is in scope, MUST observe the same plan request twice in a
temporary Git repository, a linked worktree, and a non-Git directory. The first
run MUST choose the documented destination; the second MUST update only the
same-topic active plan and MUST preserve an unrelated file and existing content.
For Git cases, MUST read back the root-anchored `git -C <root> check-ignore -v`
and the exact exclude file to confirm one effective `/.plans/` rule and no
duplicate line. For the linked worktree, MUST confirm the root and exclude paths
come from Git rather than a hard-coded `.git` directory. For the non-Git case,
MUST confirm the cwd `.plans` path and MUST report that Git exclusion is not
applicable. MUST treat these observations as proportional evidence for the
skill; MUST keep an independent semantic or installed surface evaluation
separate.
