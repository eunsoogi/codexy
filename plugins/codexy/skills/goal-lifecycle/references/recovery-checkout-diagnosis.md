# Checkout Recovery Diagnosis

## Purpose and ownership

Recovery guidance MUST separate assignment authority, checkout association, and
process execution. Codexy owns the safe sequence and evidence rules; the Codex
host owns task forking, worktree association, archival, and checkout lifetime.
If native output does not identify a cause, the owner MUST report the failed
transition and leave the cause unknown.

## Evidence boundaries

| Evidence                             | Establishes                                    | Does not establish                                         |
| ------------------------------------ | ---------------------------------------------- | ---------------------------------------------------------- |
| Native goal readback                 | The reported task's goal state and objective   | A usable checkout or successful process                    |
| Saved task CWD                       | A path recorded for the task                   | A Git worktree association or process at that path         |
| Native attachment artifact           | A host-reported association at read time       | That the checkout survives archival or can execute         |
| Git directory and worktree inventory | Git checkout identity and current registration | A native task association or successful process            |
| Scoped commit or stash               | Durable preservation of selected changes       | Checkout lifetime, ignored-data preservation, or execution |
| Exact-CWD process before archival    | Execution in the pre-archive environment       | Execution after archival                                   |
| Exact-CWD process after archival     | Execution for the observed topology and state  | A guarantee for other attachment topologies                |

A task ID, saved path, attachment record, or written procedure MUST NOT be
reported as successful execution evidence. A completed-source diagnostic MUST
NOT be described as an unfinished blocked-goal recovery.

## Diagnostic sequence

1. The recovery owner MUST preserve exact native readbacks and operation
   results. It MUST capture only the state needed to continue assigned work; a
   full transcript inventory is not a substitute for current task, goal,
   attachment, Git, and process state.
2. The owner MUST compare source and continuation task identities, exact CWD,
   native associations, canonical repository root, absolute Git directory and
   common directory, HEAD, branch state, index, tracked and untracked changes,
   and required ignored data.
3. The owner MUST treat change preservation and checkout preservation
   separately. It MUST preserve only necessary work owned by the assignment and
   resolve unclear ownership or unexpected data before any transition.
4. After source archival, the owner MUST read back the archived source, the sole
   continuation owner, its exact active objective, native checkout association,
   Git inventory, and a successful process at the intended CWD.
5. If a step fails, the owner MUST report the exact operation, native result,
   preserved state, and responsible host boundary. After an ambiguous archive
   result, the owner MUST read back source and continuation ownership before
   naming the owner; it MUST stop both from issue work when one owner cannot be
   confirmed. It MUST NOT infer a cause from a process error or turn an
   unsupported topology into a blanket claim about every checkout.

## Reusable examples

- A matching CWD and active goal do not prove that a process can start there.
- A successful attachment readback does not by itself prove post-archive
  execution.
- A commit or stash can preserve changes while the associated managed checkout
  is still removed.
- A completed-source scenario can inform only the topology it actually checks;
  it cannot prove that an unfinished blocked source can be recovered.

The ordered procedure is in
[executable-checkout-recovery.md](executable-checkout-recovery.md).
