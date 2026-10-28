# Executable-checkout recovery

This reference is part of [goal-lifecycle](../SKILL.md). It governs an
authorized same-directory recovery, not permission to create a task or clear a
goal.

The [issue #1434 diagnosis](recovery-checkout-diagnosis.md) records the observed
boundary and the outstanding native acceptance gap.

## Checkout lifetime before recovery

The
[official OpenAI worktree documentation](https://developers.openai.com/codex/app/worktrees#worktree-cleanup)
describes deletion of a managed worktree when its associated chat is archived. A
same-directory fork's CWD metadata does not establish an independent checkout
association or protect the source's checkout from that lifecycle.

Before forking or archiving a source, the recovery owner MUST:

- Capture a successful read-only process at the exact assigned CWD, with
  canonical top-level, absolute Git directory and common directory, HEAD,
  branch/detached state, staged/unstaged/untracked state, and Git worktree
  inventory. MUST preserve required ignored data without disclosing secrets.
- Capture the native source task/worktree association and exposed attachment
  identity, including an absent attachment. MUST distinguish native association,
  Git worktree registration, and a saved CWD string; none substitutes for the
  other. An absent artifact alone MUST NOT prove a missing directory.
- Identify an actually exposed, supported host operation or explicit documented
  lifetime guarantee that preserves this exact checkout for the continuation
  through source archival. MUST check its ownership and permission constraints.
  Tool discovery alone MUST NOT prove that preservation occurred. MUST retain
  actual preservation readback before archival; where it requires a fork, the
  route and authority MUST be established before the single fork and its result
  MUST be confirmed afterward, before archival.

If that route or proof is unavailable, the owner MUST stop before the unsafe
transition and report the exact gap to the parent or responsible human. MUST
retain the blocked source, unfinished goal, existing work and owner reservation.
MUST NOT invent an ownership-transfer API, use a shell Git lock as a host
lifetime guarantee, assume pinning prevents explicit archive cleanup, or create
another checkout. `attach_worktree` MUST be used only within its actual exposed
ownership constraints; it MUST NOT be assumed to transfer a source-owned
checkout. Missing capability MUST NOT become a new generic approval question.

These checks are containment. They do not repair or prove host lifetime
management. The Codex desktop fork/worktree association and archive integration
owns that boundary; Codexy owns this instruction and its evidence requirements.
MUST stop unsupported host repairs without patching the signed app or using an
undocumented app-server transport.

## Supported same-task continuation

If a supported native goal-management surface has already resumed the original
objective in the same task, the owner MUST first use native `get_goal` and apply
the exact-active continuation rule in [goal-lifecycle](../SKILL.md). MUST verify
the same task, owner and checkout with a successful exact-CWD preflight before
issue work. A matching active readback permits continuation without a fork or
source archival; it does not prove fork/archive recovery. MUST NOT infer
activation from a button click, a parent's report, or a typed command. If the
state is still blocked or differs, MUST preserve it and stop; MUST NOT replace
the objective with a command string or clear the goal. This path requires actual
supported host goal management and existing resume authority; it MUST NOT create
an invented resume API or another approval phrase.

## Ordered blocked-goal procedure

For an authorized resume, the owner MUST preserve the exact blocked-goal
readback, source task id and parent, objective, current directory, branch,
worktree, HEAD, dirty/index state, and source model and reasoning effort. The
owner MUST record requested settings from the source task assignment or creation
record separately from any effective settings the host exposes. For each
setting, the owner MUST use the effective source value when observable and
otherwise its exact requested value. If either continuation value is unavailable
from both sources, the task MUST stop before forking and report the limitation;
it MUST NOT use sender or role defaults.

1. The task MUST call `fork_thread` for the blocked task in the same directory.
   It MUST NOT create a competing branch, worktree, or task. If the call fails
   or does not return exactly one new task owner, it MUST preserve the exact
   result and stop without retrying.
2. When an authenticated source parent and terminal handoff receipt are bound to
   this task, the task MUST send exactly one receipt before source archival. The
   receipt MUST identify the fork, preserved lane state, and requested and
   effective model/effort evidence. If no source parent or receipt route is
   bound, the task MUST NOT invent a recipient, notification, or approval gate.
   If a required receipt cannot be delivered, the task MUST preserve the failure
   evidence, keep the blocked source as owner, and MUST NOT archive or continue
   the fork.
3. Only after the checkout-preservation proof above, the task MUST archive the
   original blocked task with `set_thread_archived(archived=true)`. If this
   fails, it MUST NOT send a continuation to the fork or allow either task to
   work; it MUST preserve the original owner reservation and exact error, then
   stop for parent disposition.
4. The task MUST read back that the original is archived and the fork is the
   sole active owner in the same directory before continuing. If this cannot be
   confirmed, it MUST preserve the exact readback and stop without authorizing
   task work.
5. Only after that readback, the continuation owner MUST send the fork a
   `send_message_to_thread` prompt with the source `model` and `thinking` values
   selected above. The continuation owner is the authenticated source parent
   when one exists, otherwise the current authorized task owner. The prompt MUST
   carry the same objective and require native `get_goal` before task work. If
   the tool cannot accept both values, it MUST stop without omitting either or
   relying on defaults. If delivery fails or is ambiguous, the owner MUST
   preserve the exact result, keep the fork reserved without task work, and stop
   without retrying or forking again.

In the fork, the first task action MUST be `get_goal`. An exact `null`, a
response envelope whose top-level `goal` is exactly `null`, or exact
`status=complete` permits creation of a fresh finite goal for the same
authorized objective; the fork MUST create it and read back `active`. An
`active` result permits continuation only when its objective exactly matches;
the fork MUST NOT replace it. If goal creation or its active readback fails or
differs, the fork MUST preserve the exact result and stop without task work.
Plan handling remains governed by `$orchestration` and `$planning`; this
recovery MUST NOT introduce a plan-file requirement by itself. A `blocked`
result, different active objective, error, `unknown`, `missing`, malformed
result, or other unexpected state MUST be preserved exactly and MUST stop. The
task MUST NOT fork again or retry the recovery sequence.

Requested model/effort fields prove only what the continuation requested. If the
host does not expose effective settings, the task MUST record them as
unavailable and MUST NOT claim effective model preservation was observed.
Fork/archive and goal readbacks are control-plane evidence; none proves issue,
PR, implementation, verification, review, CI, merge, release, publication, or
external-gate completion.

## Execution acceptance after source archival

After the fork's exact native goal is active, its first execution preflight MUST
run a read-only process with the exact intended CWD explicitly supplied. MUST
compare canonical top-level, absolute Git directory/common directory, HEAD,
branch/detached state and staged/unstaged/untracked state with the preserved
baseline. MUST also read back the native continuation association and Git
worktree inventory after archival. Only matching checkout identity, one
continuation owner and successful process execution permit issue work.

A process-creation failure, changed checkout, missing association/preservation
proof, or ambiguous result MUST stop issue work. MUST retain the exact error,
actual goal state, source/fork identities, baseline and reservation; report one
bounded parent update through the existing handoff contract. MUST NOT retry
forking, execute from another checkout, clear an unfinished goal, or call a
fresh replacement's success a repaired same-directory recovery. A process error
alone MUST NOT establish that archival deleted the checkout or why it failed.
MUST use actual goal-lifecycle rules for any later recovery or disposition.

## Safe native verification boundary

Without supported checkout preservation, authorized verification MUST stop at
read-only task/attachment/Git/process observations and the unavailable route.
MUST NOT fork or archive merely to observe whether the directory survives. A
genuinely completed disposable preparation goal MAY be the source of a
separately authorized scenario only after the same preservation gate passes. It
MUST NOT be fabricated from an unfinished goal or a synthetic blocked state. A
completed-source scenario can prove checkout lifetime through fork/archive; it
MUST NOT claim blocked-goal recovery, source settings preservation or any other
unobserved lifecycle behavior.

An actual recovery PASS MUST include source archival, exactly one continuation
owner, preserved checkout association and identity, and a successful process at
the intended CWD after fresh goal activation. MUST keep historical failures and
unrun acceptance criteria explicit. Repository validation, fixtures, metadata,
and fresh replacement execution MUST NOT stand in for these native readbacks.
