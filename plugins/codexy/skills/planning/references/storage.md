# Local plan storage

## Mode

Save or update a plan when the request asks to create, update, keep, or save it,
unless the user explicitly requests read-only or output-only behavior. A plain
`$planning` request uses save/update by default. Status-only questions and
output-only requests do not write.

## Destination precedence

1. Use the user's exact path when one is supplied.
2. Otherwise use one existing active plan for the same task or topic.
3. Otherwise use the default path:
   - Git project: repository root `.plans/<topic>.md`.
   - Non-Git directory: current working directory `.plans/<topic>.md`.

Use a safe, stable `<topic>` filename without path separators. Preserve an
explicit user path as given. If the target path already belongs to another
topic, or more than one active same-topic plan exists, stop before writing and
ask which path to use. A completed or archived plan is not an active plan; do
not reopen or overwrite it automatically.

## Git exclusion

For a Git project, determine the repository root and the real repository-local
exclude path before creating an untracked default plan:

```sh
git rev-parse --show-toplevel
git rev-parse --git-path info/exclude
git config --path --get core.excludesFile
git check-ignore -v --no-index -- .plans/<topic>.md
```

Use the exact `git rev-parse --git-path info/exclude` result, resolved to an
absolute path, rather than assuming `.git` is a directory. This supports a
`.git` file and linked worktree. Read existing `.gitignore`, the returned
repository exclude file, and any configured global exclude only as needed;
preserve every existing line.

If `/.plans/` is not already an effective rule, append that one rule to the
repository-local exclude file before saving the default untracked plan. Do not
add a duplicate rule, modify a shared `.gitignore` just for a local plan, or
change a tracked plan's status. If the exclude file cannot be read or written,
report `exclude not applied` and do not claim that the plan is ignored.

For a tracked or shared plan, keep the user's selected path and do not change
ignore rules. For a non-Git directory, save under its cwd `.plans` path and
report that Git exclusion is not applicable.

## Safe update

Read the target before writing. Preserve unrelated headings, user edits,
completed or archived status, and every other plan file. Replace only the
same-topic active plan fields covered by the request. Never overwrite
user-authored lines; when a user edit conflicts with the update, preserve the
original, return the new plan as output, and report the conflict. If no safe
boundary can be identified, leave the file unchanged, return the new plan as
output, and report that it was not saved.

## Verification scenarios

When storage behavior is in scope, observe the same plan request twice in a
temporary Git repository, a linked worktree, and a non-Git directory. The first
run must choose the documented destination; the second must update only the
same-topic active plan and preserve an unrelated file and existing content.
For Git cases, read back `git check-ignore -v` and the exact exclude file to
confirm one effective `/.plans/` rule and no duplicate line. For the linked
worktree, confirm the root and exclude paths come from Git rather than a
hard-coded `.git` directory. For the non-Git case, confirm the cwd `.plans`
path and report that Git exclusion is not applicable. These observations are
proportional evidence for the skill; an independent semantic or installed
surface evaluation remains a separate responsibility.
