# Checkout Recovery Diagnosis

## Purpose and ownership

Recovery guidance MUST separate assignment authority, checkout identity, and
process execution. Codexy owns the safe sequence and evidence rules; the Codex
host owns task forking, worktree association, archival, and checkout lifetime.
If native output does not identify a cause, the owner MUST report the failed
transition and leave the cause unknown.

## Evidence boundaries

| Evidence | Establishes | Does not establish |
| --- | --- | --- |
| Native goal readback | The reported task's goal state and objective | A usable checkout or successful process |
| Saved task CWD | A path recorded for that task | A Git checkout association or process at that path |
| `list_artifacts({})` result | Attachments returned for the calling chat | Another chat's checkout; an empty or PR-only result does not prove checkout absence |
| Fork result plus exact CWD and Git identity | The continuation and checkout observed at that time | Execution after source-chat archival |
| Native attachment result, when available | A host-reported association at read time | That the checkout survives archival or can execute |
| Git directory and status | Checkout identity and current changes | A native task association or successful process |
| Scoped commit or stash | Durable preservation of selected changes | Checkout lifetime, ignored-data preservation, or execution |
| Exact-CWD process before archival | Execution in the pre-archive environment | Execution after archival |
| Exact-CWD process after archival | Execution and checkout identity for the observed archived topology | A guarantee for other checkouts or attachment topologies |

A task ID, saved path, attachment record, or written procedure MUST NOT be
reported as successful execution evidence. An empty, unsupported, or failed
artifact listing MUST NOT invalidate a successful process at the exact CWD. A
completed-source diagnostic MUST NOT be described as an unfinished
blocked-goal recovery.

## Diagnostic sequence

1. The recovery owner MUST preserve exact native readbacks and operation
   results. It MUST capture only the state needed to continue assigned work; a
   full transcript inventory is not a substitute for current task, goal, Git,
   and process state.
2. The owner MUST compare source and continuation task identities, exact CWD, canonical
   repository root, absolute Git directory and common directory, HEAD, branch
   state, status, and necessary owned changes. It MUST use attachment
   information only when it is available and useful to diagnose an actual
   checkout discrepancy.
3. The owner MUST treat change preservation and checkout execution separately.
   It MUST preserve only necessary work owned by the assignment and resolve
   unclear ownership or unexpected data before a transition.
4. Before archival, the owner MUST require a successful lightweight process at
   the intended CWD and a matching Git checkout identity. It MUST NOT require a
   separate native attachment identity or lifetime guarantee when those
   observations succeed.
5. After source archival, the owner MUST read back the archived source, sole
   continuation owner, exact goal, Git identity, and successful process at the
   intended CWD.
6. If an actual CWD or checkout check fails, the owner MUST report the exact
   operation, native
   result, and preserved state; repair that observed failure when a supported
   operation permits it. An absent artifact result MUST NOT itself count as a
   failure. If ownership remains ambiguous after an archive result, the owner
   MUST stop issue work until the actual source and continuation states identify
   one owner.

## Reusable examples

- A matching CWD and active goal do not prove that a process can start there.
- An empty `list_artifacts` response from a parent does not prove a child has no
  checkout.
- A successful attachment readback does not by itself prove post-archive
  execution.
- A commit or stash can preserve changes while an associated managed checkout
  is removed.
- A completed-source scenario can inform only the topology it actually checks;
  it cannot prove that an unfinished blocked source can be recovered.

The ordered procedure is in
[executable-checkout-recovery.md](executable-checkout-recovery.md).
