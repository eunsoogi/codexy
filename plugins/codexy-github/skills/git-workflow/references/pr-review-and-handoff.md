# Pull Requests, Review, And Handoff

## PR Admission

MUST confirm the issue, exact branch/base relationship, local verification, and
repository taxonomy before opening a PR. Use a Conventional Commit title and a
body that explains the change, rationale, affected areas, verification,
evidence, omissions, follow-ups, and issue linkage without prescribing a
repository-independent heading order. Keep a PR draft while proof or known risk
is incomplete.

If selected review occurred before PR creation, the owning child MUST locally
verify one complete pre-PR import envelope and publish the Draft PR before the
first selected review for this lifecycle. The envelope MUST preserve the real
host thread, turn, final-message identity, order, reviewer facts, verdicts, and
findings; it MUST keep the current PR snapshot authoritative and MUST state that
an older imported PASS is not current-head readiness. Missing host items remain
unavailable unless the exact original host record supplies them; prose or a
synthetic historical PR snapshot is not a substitute.

Immediately read back the remote PR number, URL, title, body, state, draft
state, base, head branch, exact head SHA, labels, and linked issue. Repository
labels that apply MUST be present before readiness.

## Current Readiness State

Before every readiness or handoff claim, capture fresh authenticated GitHub
state for:

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

The handoff MUST bind the issue, branch/worktree, base, local/remote/PR head,
changed paths, verification, checks, reviews, comments, labels, issue linkage,
and unresolved threads. Ask `$orchestration` to apply its public
**completion-handoff** contract to this captured state. An intentionally open PR
MUST state the explicit parent-owned next gate; it is not merged completion.
