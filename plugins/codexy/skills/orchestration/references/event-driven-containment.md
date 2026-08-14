# Event-Driven Containment

The root/orchestrator MUST NOT autonomously poll. It MUST process only compact
deltas for terminal child state, selected-reviewer verdict, PR creation, new
HEAD, GitHub check-state change, actionable review-feedback change, or
review-thread resolution. Ordinary progress and unchanged waiting MUST NOT wake
the parent.

Every delta MUST carry a stable event identity and exact task ids. Parent-message
failure MUST emit exactly one terminal unavailable report and MUST NOT retry the
parent message. There MUST be no full conversation transfer and no full
agent-tree listing.

A parent or child MUST retain its active goal and plan during a nonterminal
external-gate wait while an implementation obligation remains. For that wait,
the child MUST use one nonterminal wait handoff with `goal state=active` and
`goal transition=none`, retain ownership, and return control when no runtime
monitor exists. It MUST NOT call `update_goal(complete)` or
`update_goal(blocked)` merely for that wait.

Once every child-owned implementation, proof, push, review-response, and
handoff obligation is actually complete, the child MAY complete its goal through
the normal terminal receipt path before runtime-only monitoring. A runtime
monitor remains runtime-owned rather than an autonomous model loop.

A registered heartbeat automation route uses its automation id, target thread,
bounded schedule, and state fingerprint; it MUST NOT require a persistent
exec/session id or same-process resume. A separate process-backed route requires
those fields plus a next deadline. Both MUST suppress unchanged observations
without assistant turns. A qualifying event MUST resume the retained goal and
plan, or start a fresh short-lived execution goal only after an earlier valid
completion. `blocked` is reserved only for an unanswered material user decision
or missing user information and MUST NOT represent an asynchronous external-gate
wait.

When a Material child event arrives—terminal child state, actionable review
feedback, or replacement-owner availability—the parent MUST validate the stable
event identity and consume it in the same turn. To consume the event, the parent
MUST perform the authorized parent-owned next action, such as routing actionable
review feedback, starting a replacement owner, or resolving a verified gate. It
MUST otherwise record a concrete execution blocker. An acknowledgement-only
output MUST NOT satisfy consumption. Duplicate stable event identities MUST
remain deduplicated with no parent action, and unchanged continuation
observations MUST NOT create assistant turns.

Orchestration MUST inspect archive candidates and the active reservation ledger
before creating a child. It MAY archive only terminal, unreferenced, clean, and
unreserved worktree lanes with no open PR or pending gate. It MUST NOT archive
PR owners or dirty/reserved candidates, and MUST record the decision in setup
evidence.

A child implementation lane MUST use a short-lived child implementation goal.
After a selected-profile BLOCK, the usable existing owner MUST record the block
and update the plan to a repair step, add faithful RED coverage when
`engineering_tdd_required` is true or proportional boundary proof otherwise,
repair, rerun terminal proof, then invoke only the permitted same-reviewer delta
recheck. A second recurrence, timeout, or UNOBSERVABLE result requires parent
decision and MUST NOT select or replace a reviewer.

MUST NOT mark a plan step complete until its evidence has been inspected. MUST
use `update_goal` only with an active or user-requested goal and current proof.
MUST reserve `blocked` for unanswered material user decisions or missing user
information.
