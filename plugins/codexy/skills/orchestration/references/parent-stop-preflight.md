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
6. For app-thread supervision, MUST read back the saved project identity and
   the actual creating tool for the Worker and any Watcher before edits. The
   Watcher MUST be in the same saved project, remain observation-only, and never
   become a second implementation owner.
7. A Watcher may carry an explicitly authorized long-lived goal, but that does
   not transfer file ownership, correction authority, final judgment, or issue
   completion to the Watcher. Record Orchestrator and Watcher lifecycle states
   separately; during an unfinished active-goal handoff, MUST NOT mark the goal
   complete merely to make the handoff fit. An observed `blocked` state remains
   governed by the existing `goal-lifecycle` recovery authority.
8. In the canonical role mapping in
   [parent-supervision.md](parent-supervision.md), the Watcher is the sole
   holder of the long-lived release goal and the Orchestrator's `get_goal`
   readback MUST be `null`. The Orchestrator MUST NOT call `create_goal` or
   recreate a goal for setup,
   callbacks, correction, review or merge decisions, or external-event resume;
   it MUST return control after authorized work. This exemption applies only to
   the Orchestrator and MUST NOT remove ordinary Worker finite-goal closure or
   `blocked` recovery.
