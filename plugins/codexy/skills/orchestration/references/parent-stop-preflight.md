# Parent Stop Preflight

MUST run this checkpoint before any implementation edit when a lane may need a
branch, worktree, PR, durable child context, or review-response ownership:

1. MUST name the atomic lane and decide ownership as `parent-owned` or
   `child-owned`.
2. If the lane is `child-owned`, the parent may prepare issue text, branch
   names, worktree requests, handoff text, and acceptance criteria, but it MUST
   NOT patch implementation files, create implementation branches or worktrees
   in the parent context, or read implementation surfaces as setup for a parent
   patch.
3. If parent draft implementation diff or setup artifacts already exist for a
   child-owned lane, MUST preserve the evidence, disclose the workflow defect,
   MUST inspect overlap with user or other-agent work, and MUST route the draft
   state to the child instead of continuing implementation.
4. When handoff or final-answer evidence for a child-owned PR includes
   parent-authored implementation, implementation setup, or review-response
   commits, MUST run the active project's child-lane ownership policy check
   against the evidence.
5. A failed first search for thread or worktree tooling is not proof that the
   tooling is unavailable. MUST continue discovery before reporting a blocker.
6. For supervision, MUST read back the saved project identity and actual
   creating tool for the Worker and the callable native-subagent tool for the
   Watcher before edits. The Watcher MUST remain observation-only and never
   become a second implementation owner.
7. The Orchestrator owns the overall active goal. A Watcher subagent may carry
   only a finite observation assignment; it MUST NOT transfer file ownership,
   correction authority, final judgement, or issue completion. Record the
   Orchestrator goal and bounded Watcher assignment separately; an observed
   `blocked` state remains governed by the existing `goal-lifecycle` recovery
   authority.
8. In the canonical role mapping in
   [parent-supervision.md](parent-supervision.md), the Orchestrator's exact
   overall goal MUST remain active while the Watcher is summoned. The Watcher
   MUST NOT create or recreate that goal, and the Orchestrator MUST NOT clear,
   transfer, or falsely complete it to fit a handoff. The Orchestrator returns
   control only after the bounded observation or material event work, while
   ordinary Worker finite-goal closure and `blocked` recovery remain required.
