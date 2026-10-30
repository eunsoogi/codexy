# Pull Requests, Review, And Handoff

## PR Admission

For a PR in an issue-sized implementation lane, MUST confirm the issue, exact
branch/base relationship, local verification, and repository taxonomy before
opening it. The PR body MUST reference the issue; issue size alone does not
select the structured body contract below. Use a Conventional Commit title and
keep a PR draft while proof or known risk is incomplete.

When the user, repository, or maintainer explicitly selects this structured
PR-body contract, the body MUST contain visible `## Summary`, `## Rationale`,
`## Changed Areas`, `## Verification`, `## Evidence`, `## Not Run`, and
`## Follow-ups` sections; the only closing reference MUST be the final line
`Fixes #<issue>`.

For ordinary authorized GitHub metadata or remote operations, follow the
[policy-selection boundary](../SKILL.md#policy-selection). The existing PR-title
and squash-subject checks remain effective on their supported paths, and
mutations use the normal host or connector route.

Prefer a selected repository template. Without one or the selected body contract
above, follow the user's format or, if none was supplied, offer a concise
`Summary` and `Verification` example. A free-form description with renamed,
omitted, or reordered headings, or in another language MUST NOT be blocked,
rewritten, or sent for extra approval by the distributed default when no
selected template or body contract applies. Review follows the selected profile,
and the installed plugin alone does not add a reviewer or approval phrase.
Source presence or replay is not proof that a host invoked a check or consumed
its denial; without request-correlated evidence, host enforcement MUST remain
unverified.

The retained PR-title check applies on supported PR creation and title-edit
paths. On those paths, the PR title MUST use `type(scope): description`, with a
nonempty valid scope and a nonempty description. An optional breaking marker
goes after the scope: `feat(task)!: change behavior`. The PR title MUST NOT
include an issue or PR number. The retained squash-subject check applies on
squash merges. The squash subject MUST be the validated PR title followed by one
ASCII space and `(#<actual PR number>)`. It MUST be added only after the
captured PR title has passed validation.

A valid PR title alone MUST NOT prove the final squash subject is valid. The
merging owner MUST follow the ordered
[final-subject sequence](merge-and-main-sync.md#authorized-squash-mutation)
immediately before merge: fresh-read title/actual PR number/head, validate the
title, build and validate the full subject, then send that same value. A child
handoff's suggested subject MUST NOT replace this fresh capture. Metadata
changes MUST invalidate the prepared request; hook silence MUST NOT count as
validation. After merging, MUST compare the remote merge commit subject with the
validated value before claiming success.

## Current-head review and ownership

The normal review path uses the current change as its evidence boundary. Read
the current PR head, relevant checks, selected reviewer result, and actual
unresolved findings. One proportionate independent reviewer is enough when the
selected profile or concrete risk calls for one. `PASS` with no actionable
findings supports the review gate; `BLOCK`, `UNOBSERVABLE`, `PENDING`,
`RUNNING`, a stale head, a failed relevant check, or an actual finding does not.

Missing historical transcripts, genesis/import records, invocation telemetry,
quota bookkeeping, and optional evaluator or connector output MUST NOT block
ordinary current-head work. Fix actionable findings in the owning child lane,
rerun relevant verification, and read the new head back. Additional reviewers,
broad rechecks, semantic evaluators, or evidence artifacts require an explicit
user or repository requirement or a concrete unresolved risk.

For a child-owned implementation lane, the owning child owns the
profile-selected reviewer when that reviewer is required and repairs findings on
the child branch. The parent consumes current-head evidence and retains merge or
publication authority. The parent MUST NOT replace the child reviewer or patch
its branch. A separately required connector review remains parent-owned and
follows the documented connector procedure.

After opening a PR, read back the remote PR number, URL, title, body, state,
draft state, base, head branch, exact head SHA, labels, and linked issue when
the selected contract requires a readiness claim. Repository labels that apply
MUST be present before that readiness claim.

## Current Readiness State

Before a readiness or handoff claim under the selected contract, capture fresh
authenticated GitHub state for:

- repository and protected default branch;
- PR number, state, draft state, merge state, base, head branch, and head SHA;
- relevant checks, selected reviews, comments, labels, and issue linkage; and
- review threads when the selected review or known feedback requires thread
  resolution evidence.

Also capture local branch status, local HEAD, and the remote-tracking head.
Those SHAs MUST equal the current PR head for a pushed/synced readiness claim.
For a stacked PR, add authenticated linked-issue evidence when GitHub does not
populate closing references.

Requested changes, actionable comments, and every unresolved actionable thread
remain blocking when that feedback is in scope. Outdated-but-fixed threads still
require current-head evidence and GitHub resolution or an accepted no-change
rationale. A green check or open PR alone is not readiness evidence, but missing
optional historical or connector evidence is not a default block.

## Child-Owned Feedback

Implementation and review-response edits stay with the branch-owning child. The
parent MUST send that owner the PR number, exact head, comment or thread URLs,
allowed paths, expected proof, and stop condition. After a repair, refresh the
PR head and checks, rerun affected verification, and confirm each thread's
current state before the parent resolves it. The parent MUST NOT patch the
child-owned branch or resolve a thread from prose alone.

## Handoff

For a child-owned implementation or an explicitly selected completion-handoff
contract, the handoff MUST use a readable Markdown table and bind the issue,
branch/worktree, base, local/remote/PR head, changed paths, relevant
verification, checks, selected review result, labels, issue linkage, and any
in-scope unresolved threads. Ask `$orchestration` to apply its public
**completion-handoff** contract to this captured state. An intentionally open PR
MUST state the explicit parent-owned next gate; it is not merged completion. An
ordinary authorized remote metadata operation does not require this handoff
shape solely because the plugin is installed.
