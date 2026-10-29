# Parent Stop Preflight

MUST run this checkpoint before any implementation edit when a lane may need a
branch, worktree, PR, durable child context, or review-response ownership:

1. MUST apply [task classification](task-classification.md#orchestration-request-ownership)
   for task kind, scoped authority, lane owner, and the distinction between
   dependencies, resource reservations, and integration conflicts. MUST record
   the atomic lane and selected owner before choosing a route. This checkpoint
   MUST NOT redefine those decisions.
2. Before choosing the current-task route or creating an app task, the parent
   MUST read back callable-tool availability, current user invocation authority
   (including explicit no-new-task or recipient limits), and the existing
   current/child owner as independent facts. The classification determines
   whether scoped orchestration authority covers a necessary bounded child task.
   A needed branch, worktree, PR, issue assignment, or task complexity MUST NOT
   by itself authorize `create_thread`.
3. When classification assigns the lane to the current task, the current-task
   route MUST continue under its native goal without creating a duplicate task.
   When classification selects a child-owned lane, the parent MUST NOT implement
   in that lane or create a duplicate owner. If ownership is unresolved or
   required authority or tooling is unavailable, implementation MUST remain
   pending and the exact gap MUST be reported. If an active child already owns
   the lane, the parent MUST send correction instructions through the supported
   task route; this does not require authority to create another task.
4. When the classified route selects an app task and current invocation limits
   and host permissions allow it, the parent MUST use the actual callable
   `create_thread` contract and verify the returned task identity, owner,
   project/worktree, and native goal before execution. The parent MUST NOT use
   an app-server/CLI bypass, fake task, or silent fallback. A text-only record
   MAY preserve the objective and exact unsupported state, but MUST NOT
   authorize execution or prove completion without the native goal.
5. If the lane is `child-owned`, the parent may prepare issue text, branch
   names, worktree requests, handoff text, and acceptance criteria, but it MUST
   NOT patch implementation files, create implementation branches or worktrees
   in the parent context, or read implementation surfaces as setup for a parent
   patch, including while child-task creation or dispatch is pending.
6. If parent draft implementation diff or setup artifacts already exist for a
   child-owned lane, MUST preserve the evidence, disclose the workflow defect,
   MUST inspect overlap with user or other-agent work, and MUST route the draft
   state to the child instead of continuing implementation.
7. When handoff or final-answer evidence for a child-owned PR includes
   parent-authored implementation, implementation setup, or review-response
   commits, MUST run the active project's child-lane ownership policy check
   against the evidence.
8. A failed first search for thread or worktree tooling is not proof that the
   tooling is unavailable. MUST continue discovery before reporting a blocker.
9. For delegated supervision, MUST read back the saved project identity and
   actual Worker creating tool, exact Worker task/host identities, and callable
   native `wait_threads` surface before edits. MUST read
   [parent-supervision.md](parent-supervision.md), the canonical source for
   role/model assignments, report routing, waits, limits, interruption,
   cancellation, and fallback.
10. The Orchestrator's exact overall goal MUST remain active; the Worker owns
    implementation and its finite goal. The Orchestrator MUST use direct
    cursor-based observation for ordinary Worker targets. Keep these surfaces
    separate and use
    [goal-transition-reporting.md](goal-transition-reporting.md) for
    blocked-goal recovery and terminal transition receipts.
11. For ordinary Worker observation, the Orchestrator MUST use native
    `wait_threads` with each target's latest cursor. A native reviewer's
    terminal delivery is separate and MUST NOT authorize Worker observation.
    MUST read [parent-supervision.md](parent-supervision.md) for host limits,
    quiet waits, interruption, cancellation, and fallback details.
12. Before implementation starts, the Orchestrator MUST give the Worker its
    exact task identity and existing authorized task-message route for required
    callbacks. Routine Worker events are observed through direct cursor-based
    waits. A direct event does not expand message authority.
