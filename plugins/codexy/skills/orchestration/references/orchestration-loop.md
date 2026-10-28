# Orchestration Loop

## Loop

1. Intake:
   - MUST use `$orchestration` before setup, delegation, implementation,
     validation, PR handling, review-response routing, or merge coordination.
   - MUST read the latest request, active project instructions, active issue,
     and relevant local skills.
   - For app-thread work, MUST read [supervision](parent-supervision.md);
     eligible hard-work elevation MUST follow
     [reconciliation](worker-elevation/reconciliation.md) before transfer. Role
     boundaries MUST stay distinct.
   - For orchestration requests, MUST apply the
     [orchestration-request ownership boundary](task-classification.md#orchestration-request-ownership):
     parent coordination and acceptance; child investigation, design, execution,
     tests, and verification. Small, urgent, or read-only work MUST NOT be
     exempt.
   - MUST separate hard requirements, preferences, assumptions, and non-goals.
   - MUST identify the observable surface that proves the request worked.
   - MUST decide separate app-task creation from branch/worktree/PR need by
     checking the callable tool, current user authority, and existing owner as
     independent facts.
   - MUST use the available `codegraph` MCP to map relevant code files and
     neighbors when the selected task has a code or repository surface.
2. Plan:
   - MUST select [`$planning`](../../planning/SKILL.md) for an explicit
     plan-creation or plan-update request, or when genuinely necessary
     decomposition is required for large work. `$planning` is the source of
     instructions for plan content, updates, and plan-file rules. Orchestration
     MUST NOT duplicate those rules or re-enter planning when it is already
     selected and in progress.
   - A plan-only request MUST return the planning result and MUST NOT create an
     issue, branch, worktree, child task, or implementation. When execution is
     authorized for one atomic issue, orchestration MUST NOT require a plan file
     or a new approval for ordinary implementation choices. An existing plan's
     completion marker MUST NOT prove current execution or verification.
   - When a plan exists for the selected planning workflow, orchestration MUST
     carry its classification, ownership, and dependency evidence into the
     planning handoff or update request before execution actions; `$planning`
     remains authoritative for the plan content and file state. An
     execution-authorized atomic issue MUST NOT receive a new plan solely for
     this handoff.
   - Keep plan state when the selected profile or lane contract requires it.
     Light and standard work MUST NOT require a visible goal/plan receipt from
     this loop alone.
   - For delegated outcomes, MUST record one Worker owner, the saved project
     identity, the Watcher identity when authorized, the callback channels, and
     one current next action. MUST NOT require a fixed checkpoint count or
     unchanged-progress receipt.
   - In [the canonical role mapping](parent-supervision.md), MUST read back the
     Orchestrator's exact active overall goal. A Watcher subagent receives only
     a bounded observation assignment and MUST NOT own or recreate that overall
     goal.
   - MUST split independent outcomes into separate issues and lanes unless a
     maintainer explicitly scopes them as one atomic lane.
   - MUST mark ownership before substantive work. Under an orchestration
     request, independent non-coordination work MUST be child-assigned under
     [child routing](child-routing-policy.md#orchestration-request-dispatch).
3. Dispatch:
   - MUST NOT dispatch until classification proves lane type, owner, atomicity,
     required skills, required tools, and first allowed action.
   - The root orchestrator MUST start specialist subagents only for bounded
     lanes without their own branch or PR.
   - Every helper or Sentinel assignment MUST include the nonrecursive
     delegation prohibition.
     `MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.`
   - For bounded helper work, the owning thread MUST route to the packaged
     specialist whose stated scope clearly matches the task surface, or record a
     concrete skip rationale. It MUST NOT count that specialist as the Codex
     child-thread/worktree owner for an issue-sized implementation lane.
   - For an explicitly requested child-owned implementation lane, the root
     orchestrator MUST start or fork a separate Codex thread in a worktree when
     the tool is available. A current-task-owned lane MUST NOT be dispatched to
     another task merely because it needs a branch, worktree, or PR.
   - For orchestration requests, MUST resolve a suitable child and authority
     before independent investigation or execution. MUST reuse existing owners.
     Generic orchestration wording MUST NOT override the actual `create_thread`
     contract or designate the parent as implementer. Missing authority or an
     unavailable child surface MUST leave that work pending; MUST NOT relabel it
     for parent execution or use unsupported CLI/app-server substitutes.
   - Ordinary Worker observation belongs to the Orchestrator, which MUST use
     native cursor-based `wait_threads` for its assigned targets. Routine
     observation MUST NOT require a Watcher relay. A separately assigned Watcher
     remains read-only and waits only on its exact targets; the Orchestrator
     awaits `watcher_wait` without double-waiting. The Worker MUST use only
     already-authorized task-message routes. A native reviewer's own terminal
     delivery is separate and MUST NOT authorize Worker observation. All wait
     ownership, deduplication, host limits, interruption, and fallback MUST
     follow [parent-supervision.md](parent-supervision.md).
   - MUST complete lane assignment before implementation edits begin. An
     Orchestrator may prepare issue text, branch name, worktree path, and
     handoff text, but MUST NOT patch implementation files for the child-owned
     lane.
   - MUST give each child an exact objective, scope, input evidence,
     deliverable, success criteria, verification, and stop/report conditions.
     Issue lanes MUST also carry issue, branch/worktree, justified path
     constraints, and read-first context; those lists define boundaries/context,
     not requested edits. A status-only child MUST NOT mask substantive parent
     execution.
   - The initial assignment MUST carry the issue outcome, executable phase
     objective and completion criteria from
     [goal-lifecycle](../../goal-lifecycle/SKILL.md), the
     [terminal handoff route](goal-transition-reporting.md), and the
     [supervised slot reservation](parent-supervision.md#supervised-app-child-capacity).
     At capacity, MUST retain ready work in the parent backlog without creating
     a child or activating its goal; batching waits MUST NOT expand capacity.
   - If parent environment validation is required after creation, MUST assign a
     short setup phase naming its check, evidence, and parent release condition.
     The parent MUST verify actual task state and permissions before releasing
     the already-scoped execution phase; the same owner MUST use actual goal
     lifecycle readback. Setup completion alone MUST NOT authorize execution.
   - A ready independent lane MUST continue through implementation,
     verification, selected review, and handoff within its existing authority.
     MUST NOT split it into routine sequential read-only, plan-approval, or
     code-only prompts. Shared migrations, Simulator reservations, other
     shared-file/resource conflicts, missing authority, and material safety
     decisions MUST retain parent gates. MUST stop only the affected work and
     continue independent authorized work; required review, routing, and slot
     limits remain binding.
   - If the current task is already explicitly assigned as implementation owner
     for the lane and no new task was explicitly requested, MUST continue in the
     current task under its native goal and MUST NOT create another app task. If
     an active child owns the lane, the parent MUST send correction instructions
     to that owner through the supported task route without requiring authority
     to create another task. The parent MUST NOT take over implementation or
     create a duplicate owner.
   - If a new app task was explicitly requested, MUST use the actual callable
     `create_thread` contract, verify the returned task and owner before
     execution, and MUST NOT use an app-server/CLI bypass or fake task.
4. Integrate:
   - MUST process actionable compact child results before re-reading
     transcripts; MUST verify the linked current artifacts and missing or
     contradictory proof. Parent acceptance MUST stay within minimum existing
     artifact/diff/evidence readbacks. New investigation, reproduction, or test
     execution MUST go to the owner or a bounded investigation child.
   - MUST preserve user changes and unrelated work.
   - MUST treat Worker callbacks and Watcher observations as signals, not
     acceptance. For drift, the Orchestrator MUST compare the observed evidence
     with current scope, ownership, and user constraints. It MUST give
     criteria-based direction; the Worker MUST choose and implement repairs. The
     Orchestrator MUST verify the next relevant call, diff, or result and
     preserve the creating surface and native review history.
   - MUST resolve cross-lane conflicts in the orchestrator thread and route
     child-owned issue findings to the owning Worker under
     [issue-owned implementation decisions](task-classification.md#issue-owned-implementation-decisions).
     Feedback MUST identify an unmet criterion or verified constraint and its
     evidence gap; the Worker chooses the repair.
   - If the child owner stops responding, MUST stop and report the PR head,
     owner, last contact, and required evidence. MUST NOT recover by patching
     the branch unless a maintainer explicitly reassigns implementation
     ownership.
5. MUST verify:
   - Let the selected references own the obligation: engineering behavior
     follows `$engineering`, instruction-only and documentation work uses
     proportional structural readback, review references own current-head
     review, and `$proof-driven-completion` owns final claims. This loop MUST
     NOT add a universal test, review, or evidence checklist.
   - MUST run local checks in the owning worktree.
   - Execution verification MUST run in the owning child/worktree for an
     orchestration request; parent acceptance MUST judge its returned proof.
   - The assigned execution owner MUST drive the selected external surface when
     changing GitHub, browser/desktop, CLI, plugin, marketplace,
     documents/artifacts, spreadsheets/data, research/wiki, or project settings
     behavior.
   - MUST keep evidence tied to the exact commit, PR head, file state, or
     runtime surface being claimed.
   - An app-task creation claim MUST use actual host creation and ownership
     readback; source tests, configuration, or app-server transcripts MUST NOT
     substitute for that evidence. If a required native goal or host route is
     unavailable, MUST report the exact limitation and required action without
     claiming execution progress or completion from text alone.
   - Direct observation claims MUST use the actual `wait_threads` surface with
     assigned targets and current cursors. When a separate Watcher assignment is
     in scope, exercise its real subagent creation, Worker message route,
     `watcher_report`/`watcher_wait`, and Worker readback. Keep the Orchestrator
     goal separate from any bounded Watcher assignment; no event proves issue
     completion.
6. Finish:
   - MUST confirm no running sessions, open child lanes, untracked required
     files, or unverified claims remain.
   - MUST confirm no final-answer or handoff artifact claims completion while a
     matching clean PR remains open unless the maintainer explicitly requested
     stop, wait, draft-only, or leave-open behavior.
   - MUST report what changed, what proved it, what was not run, and remaining
     risk.
   - MUST report the current request, callable task contract, selected route,
     actual owner, and execution result separately.
   - MUST report measured and unmeasured Worker/Watcher usage separately, retain
     actual review history, and state unsupported wake, transfer, or recovery
     behavior instead of inventing it.

## Failure Modes

- Starting setup, delegation, implementation, validation, PR handling, review
  response, or merge coordination before `$orchestration`.
- Treating subagents as child-owned Codex thread/worktree owners.
- Marking a goal blocked because review, child work, worktree/thread setup, or
  another asynchronous tool is pending.
- Treating expected or registered MCPs as ordinary unavailable tools when the
  callable Codex surface does not expose them.
- Starting parent implementation patches for a lane that needs its own child
  thread, worktree, branch, or PR, then delegating only after files changed.
- Treating parent-only readthrough, arbitrary reviewer agents, generic review
  roles, or stale reviewer output as the packaged Codexy reviewer gate.
- Reporting completion while review comments, open threads, stale PR heads, or
  unverified claims remain unresolved.

## Multi-Agent Dispatch Template

```text
Lane goal / success criteria:
Task classification:
Atomic lane:
Issue:
Branch:
Worktree path:
Path constraints/reservations and reason (Worker selects files):
Read-first context (Worker chooses implementation files):
Deliverable:
Verification:
Required evidence:
Classification evidence:
Review feedback route:
Parent acceptance: criteria/evidence; Worker chooses repairs
Initial phase contract: issue outcome; Assignment objective; phase criteria
Capacity: reserved slot; shared-file/resource reservations; parent gates
Setup phase: only if required; check, evidence, release condition
Completion route: goal-transition-reporting.md; compact current-head result
Return evidence: selected route/profile only; exact head, checks, findings,
  external gates, and one parent-owned next action
  Strict/audited lanes: native goal and active readback; todo/plan state or
  unavailable fallback; touched LOC; Codegraph/LSP evidence or applicability;
  multi-agent use or concrete scope/tool rationale; current-head packaged review.
  Light/standard: only selected-route evidence; light has no LLM reviewer.
Child execution discipline:
Stop if:
```

## Codex Thread And Worktree Handoff

For any lane needing its own branch, PR, or long-running implementation context,
MUST use the dispatch template above as its first message and retain the same
owner for follow-up work. The return MUST include native goal readback and
todo/plan fallback; multi-agent use or rationale; Codegraph/LSP for code;
touched LOC for non-trivial code/harness/workflow rules; and the current-head
packaged reviewer result when selected (strict: Sentinel).

The child thread MUST NOT merge, close issues, or claim final completion. It
returns evidence and a commit-ready branch to the invoking orchestrator thread.
