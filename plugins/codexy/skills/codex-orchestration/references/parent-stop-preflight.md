# Parent Stop Preflight

MUST run this checkpoint before any implementation edit when a lane may need a
branch, worktree, PR, durable child context, or review-response ownership:

1. MUST name the atomic lane and decide ownership as `parent-owned` or
   `child-owned`.
2. If the lane is `child-owned`, the parent may prepare issue text, branch
   names, worktree requests, handoff text, and acceptance criteria, but it
   MUST NOT patch implementation files, create implementation branches or
   worktrees in the parent context, or read implementation surfaces as setup
   for a parent patch.
3. If parent draft implementation diff or setup artifacts already exist for a
   child-owned lane, MUST preserve the evidence, disclose the workflow defect,
   MUST inspect overlap with user or other-agent work, and MUST route the draft
   state to the child instead of continuing implementation.
4. When handoff or final-answer evidence for a child-owned PR includes
   parent-authored implementation, implementation setup, or review-response
   commits, MUST run
   `scripts/validate-plugin-config --check-child-lane-ownership --evidence-file <path>`.
5. A failed first search for thread or worktree tooling is not proof that the
   tooling is unavailable. MUST continue discovery before reporting a blocker.
