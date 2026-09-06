# Pull Requests, Review, And Handoff

## PR Admission

For a PR that belongs to an issue-sized implementation lane, or when this
repository or maintainer explicitly selects the following PR contract, MUST
confirm the issue, exact branch/base relationship, local verification, and
repository taxonomy before opening it. Use a Conventional Commit title and a
body with visible `## Summary`, `## Rationale`, `## Changed Areas`,
`## Verification`, `## Evidence`, `## Not Run`, and `## Follow-ups` sections;
the only closing reference MUST be the final line `Fixes #<issue>`. Keep a PR
draft while proof or known risk is incomplete.

For ordinary authorized GitHub metadata or remote operations, including an
issue/PR update, review, workflow, release, or merge request, do not require a
new issue, local branch/worktree, prescribed title/body/footer, transcript
reconstruction, or diagnostic invocation solely because this plugin is
installed. Follow the actual user or repository choice and the host, connector,
and GitHub response.

Prefer an existing repository template when one is selected. If none is
selected, offer a concise `Summary` and `Verification` example and expand it
only as needed. An adequate free-form description, renamed/omitted/reordered
sections, or another language or title style MUST NOT be blocked, rewritten, or
sent for extra approval by the distributed default.

When the PR contract above is selected, the PR title MUST use
`type(scope): description`, with a nonempty valid scope and a nonempty
description. An optional breaking marker goes after the scope:
`feat(task)!: change behavior`. The PR title MUST NOT include an issue or PR
number. The squash subject MUST be the validated PR title followed by one ASCII
space and `(#<actual PR number>)`. It MUST be added only after the captured PR
title has passed validation. Otherwise, use the actual user or repository title
and merge-message convention.

Native host transcript capture and recovery apply only when a selected review or
transition path consumes those historical events. If that path uses a selected
review that occurred before PR creation, the owning child MUST locally verify
one complete pre-PR import envelope and publish the Draft PR before the first
selected review for this lifecycle. The envelope MUST preserve the real host
thread, turn, final-message identity, order, reviewer facts, verdicts, and
findings; it MUST keep the current PR snapshot authoritative and MUST state that
an older imported PASS is not current-head readiness. Missing host items remain
unavailable unless the exact original host record supplies them; prose or a
synthetic historical PR snapshot is not a substitute.

If selected review events completed after PR creation and the selected path
consumes them, the owning child MUST capture the supported native host records
before recovery: the complete owner page chain containing the single reviewer
`spawnAgent` or `spawn_agent`, the matching reviewer page chain, continuation
cursors, completed final messages, source-local order, actual model/effort,
reviewed heads, terminal results, findings, timestamps, and unchanged raw UTF-8
text. The child MUST run the existing
`codexy-review-control --recover-native-review-history` mode with the fresh
authenticated current PR snapshot and keep the input capture outside tracked
files. The mode produces a non-admitted top-level `nativeHistoryRecovery`
receipt; it does not authenticate caller fields, invent a historical snapshot,
or establish current-head readiness. The next build MUST carry that receipt
forward and consume the recovered predecessor through the ordinary transition
validator. It MAY remove `native_history_recovery` only while appending a real
current-head verdict; it MUST retain and revalidate
`native_history_provenance`, the full/delta event prefix, actual source reviewer
facts, findings, and event counts. Direct recovery output MUST NOT be described
as PR-ready, complete, merge-authorized, or evidence that another review is
needed.

After opening a PR, read back the remote PR number, URL, title, body, state,
draft state, base, head branch, exact head SHA, labels, and linked issue when
the selected contract requires a readiness claim. Repository labels that apply
MUST be present before that readiness claim.

## Current Readiness State

Before every readiness or handoff claim under the selected contract, capture
fresh authenticated GitHub state for:

- repository and protected default branch;
- PR number, state, draft state, merge state, base, head branch, and head SHA;
- checks, reviews, latest reviews, comments, labels, and issue linkage; and
- all review threads with resolution, outdated state, path, comment URL, author,
  body, creation time, and comment commit SHA.

Also capture local branch status, local HEAD, and the remote-tracking head.
Those SHAs MUST equal the current PR head for a pushed/synced readiness claim.
For a stacked PR, add authenticated linked-issue evidence when GitHub does not
populate closing references.

Requested changes, actionable comments, and every unresolved actionable thread
remain blocking. Outdated-but-fixed threads still require current-head evidence
and GitHub resolution or an accepted no-change rationale. A green check or open
PR alone is not readiness evidence.

## Child-Owned Feedback

Implementation and review-response edits stay with the branch-owning child. The
parent MUST send that owner the PR number, exact head, comment or thread URLs,
allowed paths, expected proof, and stop condition. After a repair, refresh the
PR head and checks, rerun affected verification, and confirm each thread's
current state before the parent resolves it. The parent MUST NOT patch the
child-owned branch or resolve a thread from prose alone.

## Handoff

For a child-owned implementation or an explicitly selected completion-handoff
contract, the handoff MUST bind the issue, branch/worktree, base, local/remote/PR
head, changed paths, verification, checks, reviews, comments, labels, issue
linkage, and unresolved threads. Ask `$orchestration` to apply its public
**completion-handoff** contract to this captured state. An intentionally open PR
MUST state the explicit parent-owned next gate; it is not merged completion.
An ordinary authorized remote metadata operation does not require this handoff
shape solely because the plugin is installed.
