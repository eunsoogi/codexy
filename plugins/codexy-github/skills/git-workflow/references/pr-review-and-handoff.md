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
new issue, local branch/worktree, prescribed body/footer, transcript
reconstruction, or diagnostic invocation solely because this plugin is
installed. The existing PR-title and squash-subject checks remain effective on
their supported paths; otherwise follow the actual user or repository choice and
the host, connector, and GitHub response.

Prefer an existing repository template when one is selected. If none is
selected, offer a concise `Summary` and `Verification` example and expand it
only as needed. The installed component separately retains its existing PR-title
check on supported creation and edit paths and its squash-subject check on
squash merges. Those checks are limited to the existing title contracts and do
not impose a body template, review quota, or fixed approval phrase. An adequate
free-form description, renamed/omitted/reordered sections, or another language
MUST NOT be blocked, rewritten, or sent for extra approval by the distributed
default.

The retained PR-title check applies on supported PR creation and title-edit
paths. On those paths, the PR title MUST use `type(scope): description`, with a
nonempty valid scope and a nonempty description. An optional breaking marker
goes after the scope: `feat(task)!: change behavior`. The PR title MUST NOT
include an issue or PR number. The retained squash-subject check applies on
squash merges. The squash subject MUST be the validated PR title followed by one
ASCII space and `(#<actual PR number>)`. It MUST be added only after the
captured PR title has passed validation. These retained title checks do not
impose a body template, review quota, fixed approval phrase, or exclusive
mutation route.

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
current-head verdict; it MUST retain and revalidate `native_history_provenance`,
the full/delta event prefix, actual source reviewer facts, findings, and event
counts. Direct recovery output MUST NOT be described as PR-ready, complete,
merge-authorized, or evidence that another review is needed.

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
contract, the handoff MUST bind the issue, branch/worktree, base,
local/remote/PR head, changed paths, relevant verification, checks, selected
review result, labels, issue linkage, and any in-scope unresolved threads. Ask
`$orchestration` to apply its public **completion-handoff** contract to this
captured state. An intentionally open PR MUST state the explicit parent-owned
next gate; it is not merged completion. An ordinary authorized remote metadata
operation does not require this handoff shape solely because the plugin is
installed.
