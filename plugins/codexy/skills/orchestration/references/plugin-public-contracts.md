# Public extension contracts

- `issue-intake receipt`: validate the canonical receipt and require parent
  approval before issue mutation; unsupported observations remain handoff-only.
- `child-lane-ownership`: verify the named child owns its issue-sized
  branch/worktree lane before accepting implementation evidence.
- `completion-handoff`: require current PR, review-thread, head, local/remote,
  label, and active-project handoff evidence before a terminal claim.

Extensions MUST invoke these named contracts and MUST NOT derive private core
paths or substitute missing proof.
