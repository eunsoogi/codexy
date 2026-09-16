# Parent Stop Preflight

MUST run this checkpoint before any implementation edit when a lane may need a
branch, worktree, PR, durable child context, or review-response ownership:

1. MUST name the atomic lane and decide ownership as `parent-owned` or
   `child-owned`.
2. Before selecting a current-task route or creating a separate app task, the
   parent MUST record tool availability, current user invocation authority, and
   existing current/child owner as three independent facts. A needed branch,
   worktree, PR, issue assignment, or task complexity MUST NOT by itself
   authorize `create_thread`.
3. If the current task owns the lane and no separate task was explicitly
   requested, the current-task route MUST continue under its native goal and
   MUST NOT create another app task. If an active child already owns the lane,
   the parent MUST send correction instructions through the supported task route
   and MUST NOT implement in the parent or create a duplicate owner.
4. If a separate task was explicitly requested, the parent MUST use the actual
   callable `create_thread` contract and verify the returned task identity,
   owner, project/worktree, and native goal before execution. The parent MUST
   NOT use an app-server/CLI bypass, fake task, or silent fallback. A text-only
   record MAY preserve the objective and exact unsupported state, but it MUST
   NOT authorize execution or prove completion without the native goal.
5. If the lane is `child-owned`, the parent may prepare issue text, branch
   names, worktree requests, handoff text, and acceptance criteria, but it MUST
   NOT patch implementation files, create implementation branches or worktrees
   in the parent context, or read implementation surfaces as setup for a parent
   patch.
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
   actual Worker creating tool, plus the callable native-subagent tool and exact
   Watcher identity before edits. MUST read
   [parent-supervision.md](parent-supervision.md), the canonical source for
   role/model assignments, report routing, waits, limits, interruption, and
   fallback. The Watcher MUST remain observation-only and MUST NOT become
   another implementation owner.
10. The Orchestrator's exact overall goal MUST remain active; the Watcher MAY
    receive only a bounded observation assignment; the Worker owns
    implementation and its finite goal. The Orchestrator MUST keep these
    surfaces separate and use
    [goal-transition-reporting.md](goal-transition-reporting.md) for goal and
    terminal receipts.
11. Only the assigned Watcher MAY call `wait_threads` for Worker targets. The
    Orchestrator MUST await `watcher_wait` and MUST NOT directly wait, retry, or
    poll them. A native reviewer's terminal delivery is a separate surface and
    MUST NOT authorize Worker observation. MUST read
    [parent-supervision.md](parent-supervision.md) before this wait route for
    host-limit, quiet-wait, interruption, cancellation, and fallback details.
12. Before implementation starts, the Orchestrator MUST give the Worker the
    exact Watcher task and supported task-message route for ordinary reports.
    The Worker MUST NOT receive a Watcher session token or call Watcher MCP
    transport tools. A verified unavailable route or concrete emergency permits
    one marked direct Orchestrator fallback, not routine duplicate reporting.
